#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod fetch;
mod html;
mod settings;
mod url;
mod canvas;

use aidoku::{
    BaseUrlProvider, Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, ImageResponse, Manga, MangaPageResult, Page, PageContext, 
    PageImageProcessor, Result, Source, 
    alloc::{String, Vec, string::ToString as _}, 
    canvas::Rect, 
    imports::canvas::{Canvas, ImageRef}, 
    prelude::*
};

use crate::{canvas::decode_jm_canvas_data, fetch::Fetch};
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
            response: ImageResponse,
            context: Option<PageContext>,
    ) -> Result<ImageRef> {
        let context = match context {
            Some(ctx) => ctx,
            None => return Ok(response.image),
        };

        let canvas_data_b64 = context.get("canvas_data").cloned().unwrap_or_default();
        if canvas_data_b64.is_empty() {
            return Ok(response.image);
        }

        // 呼叫我們的純函式進行解析
        let scramble_data = match decode_jm_canvas_data(&canvas_data_b64) {
            Some(data) => data,
            None => return Ok(response.image), // 解析失敗就回傳原圖
        };

        // 直接使用解析出來的寬高建立畫布
        let mut canvas = Canvas::new(scramble_data.width, scramble_data.height);

        // 走訪座標進行拼貼
        for arg in scramble_data.args {
            let src_rect = Rect::new(arg[0], arg[1], arg[2], arg[3]);
            let des_rect = Rect::new(arg[4], arg[5], arg[6], arg[7]);
            canvas.copy_image(&response.image, src_rect, des_rect);
        }

        Ok(canvas.get_image())
    }
}

register_source!(Jmtt, DeepLinkHandler, BaseUrlProvider, PageImageProcessor);

#[cfg(test)]
mod test;