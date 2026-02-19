#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod fetch;
mod html;
mod settings;
mod url;

use aidoku::{
    BaseUrlProvider, Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Manga, MangaPageResult, PageImageProcessor,
    Page, Result, Source,
    imports::canvas::ImageRef,
    alloc::{String, Vec, string::ToString as _},
    prelude::*,
};

use crate::fetch::Fetch;
use crate::html::GenManga;
use crate::url::Url;

struct Jmtt;

impl Source for Jmtt {
    fn new() -> Self {
        Self
    }

    fn get_search_manga_list(
        &self,
        query: Option<String>,
        page: i32,
        filters: Vec<FilterValue>,
    ) -> Result<MangaPageResult> {
        let url = Url::filters(query.as_deref(), page, &filters)?.to_string();

        let response = Fetch::get(url)?.get_html()?;

        GenManga::list(&response)
    }

    fn get_manga_update(
        &self,
        mut manga: Manga,
        needs_details: bool,
        needs_chapters: bool,
    ) -> Result<Manga> {
        let url = Url::book(manga.key.clone())?.to_string();

        let response = Fetch::get(url)?.get_html()?;

        if needs_details {
            GenManga::detail(&response, &mut manga)?;
        }

        if needs_chapters {
            manga.chapters = Some(GenManga::chapters(&response)?);
        }

        Ok(manga)
    }

    fn get_page_list(&self, _: Manga, chapter: Chapter) -> Result<Vec<Page>> {
        let url = Url::chapter(chapter.key.clone())?.to_string();

        let response = Fetch::get(url)?.get_html()?;

        GenManga::chapter(&response)
    }
}

impl DeepLinkHandler for Jmtt {
    fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
        if url.contains("/album/") {
            let key = url.split("/").skip(4).next().unwrap_or_default().to_string();

            if key.is_empty() {
                return Ok(None);
            }

            return Ok(Some(DeepLinkResult::Manga { key }));
        }

        Ok(None)
    }
}

impl BaseUrlProvider for Jmtt {
    fn get_base_url(&self) -> Result<String> {
        Ok(settings::get_base_url())
    }
}

impl PageImageProcessor for Jmtt {
    fn process_page_image(
            &self,
            response: aidoku::ImageResponse,
            context: Option<aidoku::PageContext>,
    ) -> Result<ImageRef> {
        let Some(context) = context else {
            return Ok(response.image);
        };

        let Some(key) = context.get("key") else {
            bail!("Missing encryption key");
        };

        let data = response.image.data();

        let key_stream: core::result::Result<Vec<u8>, core::num::ParseIntError> = key
            .as_bytes()
            .chunks(2)
            .map(|chunk| {
                let s = core::str::from_utf8(chunk).unwrap();
                u8::from_str_radix(s, 16)
            })
            .collect();

        let Ok(key_stream) = key_stream else {
            bail!("Invalid encryption key");
        };

        let decoded: Vec<u8> = data
            .iter()
            .enumerate()
            .map(|(i, &byte)| byte ^ key_stream[i % key_stream.len()])
            .collect();

        Ok(ImageRef::new(&decoded))
    }
}

register_source!(Jmtt, DeepLinkHandler, BaseUrlProvider, PageImageProcessor);

#[cfg(test)]
mod test;