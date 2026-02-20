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
        _context: Option<PageContext>
    ) -> Result<ImageRef> {
        // 從請求 URL 中解析 pieces 參數（由 chapter() 附加的 &pieces=N）
        let pieces: u32 = response.request.url
            .as_deref()
            .and_then(|url| {
                url.split('&')
                    .find(|s: &&str| s.starts_with("pieces="))
                    .and_then(|s| s["pieces=".len()..].parse().ok())
            })
            .unwrap_or(0);

        // pieces <= 1 代表此圖片不需要重排（非 WebP 或無混淆）
        if pieces <= 1 {
            return Ok(response.image);
        }

        let image  = &response.image;
        let width  = image.width();
        let height = image.height();

        // 建立與原圖相同大小的畫布
        let mut canvas = Canvas::new(width, height);

        // 計算每片高度與餘數（與 JS 邏輯相同）
        let remainder = (height as u32) % pieces;
        let slice_h   = (height / pieces as f32).floor();

        for i in 0..pieces {
            let mut src_y = slice_h * i as f32;
            // 打亂後的來源 Y 座標（從圖片底部往上算）
            let dst_y     = height - slice_h * (i + 1) as f32 - remainder as f32;
            let mut cur_h = slice_h;

            // 第一片補上餘數高度
            if i == 0 {
                cur_h += remainder as f32;
            } else {
                src_y += remainder as f32;
            }

            // 將打亂位置的切片複製到正確位置
            // src_rect = 打亂圖的位置，dst_rect = 還原後的正確位置
            canvas.copy_image(
                image,
                Rect::new(0.0, dst_y, width, cur_h),
                Rect::new(0.0, src_y, width, cur_h),
            );
        }

        Ok(canvas.get_image())
    }
}

register_source!(Jmtt, DeepLinkHandler, BaseUrlProvider, PageImageProcessor);

#[cfg(test)]
mod test;
