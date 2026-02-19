#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod fetch;
mod html;
mod settings;
mod url;

use aidoku::{
    BaseUrlProvider, Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, ImageResponse, Manga, MangaPageResult, Page, PageContext, 
    PageImageProcessor, Result, Source, 
    alloc::{String, Vec, string::ToString as _}, 
    canvas::Rect, 
    imports::canvas::{Canvas, ImageRef}, 
    prelude::*
};

use base64::{engine::general_purpose, Engine};

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
            response: ImageResponse,
            context: Option<PageContext>,
    ) -> Result<ImageRef> {
        // 1. 取出 Context
        let context = match context {
            Some(ctx) => ctx,
            None => return Ok(response.image),
        };

        // 2. 取出 Base64 字串
        let canvas_data_b64 = match context.get("canvas_data") {
            Some(data) if !data.is_empty() => data,
            _ => return Ok(response.image),
        };

        // 3. 解碼 Base64
        let decoded_bytes = match general_purpose::STANDARD.decode(canvas_data_b64) {
            Ok(b) => b,
            Err(_) => return Ok(response.image),
        };
        let json_str = String::from_utf8(decoded_bytes).unwrap_or_default();

        // 4. 手動字串解析 JSON (不依賴任何外部 JSON 套件，效能最高！)
        // JSON 範例: {"url":"...","args":[[0,415,720,85,0,0,720,85],...],"width":720,"height":500}
        
        // 擷取 width
        let width: f32 = json_str.split("\"width\":")
            .nth(1).unwrap_or("0")
            .split(',')
            .next().unwrap_or("0")
            .trim().parse().unwrap_or(0.0);

        // 擷取 height
        let height: f32 = json_str.split("\"height\":")
            .nth(1).unwrap_or("0")
            .split('}')
            .next().unwrap_or("0")
            .trim().parse().unwrap_or(0.0);

        // 如果解析失敗，直接回傳原圖
        if width == 0.0 || height == 0.0 {
            return Ok(response.image);
        }

        // 5. 建立畫布準備拼圖
        let mut canvas = Canvas::new(width, height);

        // 6. 擷取 args 陣列並拼圖
        if let Some(args_part) = json_str.split("\"args\":[").nth(1).and_then(|s| s.split("],\"").next()) {
            // 迴圈走訪每個區塊，例如: 0,415,720,85,0,0,720,85
            for arg_group in args_part.split("],[") {
                let clean_group = arg_group.replace('[', "").replace(']', "");
                let nums: Vec<f32> = clean_group
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();

                // 確保座標數量正確 (來源 x,y,w,h 與 目標 x,y,w,h)
                if nums.len() >= 8 {
                    let src_rect = Rect::new(nums[0], nums[1], nums[2], nums[3]);
                    let des_rect = Rect::new(nums[4], nums[5], nums[6], nums[7]);
                    canvas.copy_image(&response.image, src_rect, des_rect);
                }
            }
        }

        // 7. 回傳拼好的圖片
        Ok(canvas.get_image())
    }
}

register_source!(Jmtt, DeepLinkHandler, BaseUrlProvider, PageImageProcessor);

#[cfg(test)]
mod test;