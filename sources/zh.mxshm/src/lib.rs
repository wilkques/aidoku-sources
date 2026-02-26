#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod fetch;
mod home;
mod html;
mod settings;
mod url;

use aidoku::{
    BaseUrlProvider, Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Listing,
    ListingProvider, Manga, MangaPageResult, Page, Result, Source,
    alloc::{String, Vec, string::ToString as _, vec},
    prelude::*,
};

use crate::fetch::Fetch;
use crate::html::GenManga;
use crate::url::Url;

struct Mxshm;

impl Source for Mxshm {
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

        let response = Fetch::get(url)?.html()?;

        GenManga::list(&response)
    }

    fn get_manga_update(
        &self,
        mut manga: Manga,
        needs_details: bool,
        needs_chapters: bool,
    ) -> Result<Manga> {
        let url = Url::book(manga.key.clone())?.to_string();

        let response = Fetch::get(url)?.html()?;

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

        let response = Fetch::get(url)?.html()?;

        GenManga::chapter(&response)
    }
}

impl DeepLinkHandler for Mxshm {
    fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
        if url.contains("/book/") {
            let key = url.split("/book/").last().unwrap_or_default().to_string();

            if key.is_empty() {
                return Ok(None);
            }

            return Ok(Some(DeepLinkResult::Manga { key }));
        }

        Ok(None)
    }
}

impl BaseUrlProvider for Mxshm {
    fn get_base_url(&self) -> Result<String> {
        Ok(settings::get_base_url())
    }
}

impl ListingProvider for Mxshm {
    fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
        let filters = match listing.id.as_str() {
            "dailymanga" => vec![FilterValue::Select {
                id: "列表".to_string(),
                value: "update".to_string(),
            }],
            "popularitymanga" => vec![FilterValue::Select {
                id: "列表".to_string(),
                value: "update".to_string(),
            }],
            "finishmanga" => vec![FilterValue::Select {
                id: "进度".to_string(),
                value: "0".to_string(),
            }],
            "recommendmanga" => vec![FilterValue::Select {
                id: "列表".to_string(),
                value: "update".to_string(),
            }],
            _ => bail!("Invalid listing"),
        };

        let url = Url::filters(None, page, &filters)?.to_string();

        let response = Fetch::get(url)?.html()?;

        GenManga::list(&response)
    }
}

register_source!(
    Mxshm,
    DeepLinkHandler,
    BaseUrlProvider,
    Home,
    ListingProvider
);

#[cfg(test)]
mod test;
