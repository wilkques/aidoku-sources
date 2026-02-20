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
        // JS: var nW = img.naturalWidth; var nH = img.naturalHeight;
        let nw = image.width();
        let nh = image.height() as u32;

        // JS: canvas.width = nW; canvas.height = nH;
        let mut canvas = Canvas::new(nw, nh as f32);

        // JS: var remainder = nH % pieces;
        let remainder = nh % pieces;

        // JS: for (var m = 0; m < pieces; m++) { ... }
        for m in 0..pieces {
            // JS: var h = Math.floor(nH / pieces);
            let mut h = nh / pieces;

            // JS: var srcY = h * m;
            let mut src_y = h * m;

            // JS: var dstY = nH - h * (m + 1) - remainder;
            let dst_y = nh - h * (m + 1) - remainder;

            // JS: if (m == 0) h += remainder; else srcY += remainder;
            if m == 0 {
                h += remainder;
            } else {
                src_y += remainder;
            }

            // JS: ctx.drawImage(img, 0, dstY, nW, h, 0, srcY, nW, h);
            //     drawImage(img, sx, sy,  sw, sh, dx, dy,   dw, dh)
            //
            // Aidoku: copy_image(image, src_rect, dst_rect)
            //   src_rect = 從打亂圖讀取的區域 (sx=0, sy=dstY, sw=nW, sh=h)
            //   dst_rect = 畫到畫布的區域     (dx=0, dy=srcY, dw=nW, dh=h)
            canvas.copy_image(
                image,
                Rect::new(0.0, dst_y as f32, nw, h as f32),
                Rect::new(0.0, src_y as f32, nw, h as f32),
            );
        }

        Ok(canvas.get_image())
    }
}


register_source!(Jmtt, DeepLinkHandler, BaseUrlProvider, PageImageProcessor);

#[cfg(test)]
mod test;
