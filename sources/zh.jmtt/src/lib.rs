#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod fetch;
mod html;
mod settings;
mod url;
mod image;

use aidoku::{
    BaseUrlProvider,
    Chapter,
    DeepLinkHandler,
    DeepLinkResult,
    FilterValue,
    ImageRequestProvider,
    ImageResponse,
    Manga,
    MangaPageResult,
    Page,
    PageContext,
    PageImageProcessor,
    Result,
    Source,
    alloc::{ String, Vec, string::ToString as _ },
    canvas::Rect,
    imports::canvas::{ Canvas, ImageRef },
    imports::net::Request,
    prelude::*,
};

use crate::fetch::{ Client, Fetch };
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
        filters: Vec<FilterValue>
    ) -> Result<MangaPageResult> {
        let url = Url::filters(query.as_deref(), page, &filters)?.to_string();

        let response = Fetch::get(url)?.get_html()?;

        GenManga::list(&response)
    }

    fn get_manga_update(
        &self,
        mut manga: Manga,
        needs_details: bool,
        needs_chapters: bool
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

        GenManga::chapter(&response, &chapter.key)
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
        response: ImageResponse,
        context: Option<PageContext>
    ) -> Result<ImageRef> {
        // 從 PageContext (HashMap) 中讀取 pieces 參數
        let pieces: u32 = context
            .and_then(|ctx| ctx.get("pieces").and_then(|v| v.parse().ok()))
            .unwrap_or(0);

        // pieces <= 1 代表此圖片不需要重排
        if pieces <= 1 {
            return Ok(response.image);
        }

        let image = &response.image;
        let width = image.width(); // JS: var width = img.naturalWidth;
        let height = image.height() as u32; // JS: var height = img.naturalHeight;

        let mut canvas = Canvas::new(width, height as f32);

        let remainder = height % pieces;

        // JS loop port
        for i in 0..pieces {
            let mut slice_height = height / pieces;
            let mut src_y = slice_height * i;
            let dest_y = height - slice_height * (i + 1) - remainder;

            if i == 0 {
                slice_height += remainder;
            } else {
                src_y += remainder;
            }

            // Copy the strip from source to destination
            // Aidoku copy_image(image, src_rect, dest_rect)
            canvas.copy_image(
                image,
                Rect::new(0.0, dest_y as f32, width, slice_height as f32), // src_rect: 從打亂圖抓取
                Rect::new(0.0, src_y as f32, width, slice_height as f32) // dst_rect: 覆寫回預期正確位置
            );
        }

        Ok(canvas.get_image())
    }
}

impl ImageRequestProvider for Jmtt {
    fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
        Ok(Client::get(url)?)
    }
}

register_source!(Jmtt, DeepLinkHandler, BaseUrlProvider, PageImageProcessor, ImageRequestProvider);

#[cfg(test)]
mod test;
