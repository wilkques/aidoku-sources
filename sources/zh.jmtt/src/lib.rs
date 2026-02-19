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
        let context = match context {
            Some(ctx) => ctx,
            None => return Ok(response.image),
        };

        let canvas_data_b64 = match context.get("canvas_data") {
            Some(data) if !data.is_empty() => data,
            _ => return Ok(response.image),
        };

        let decoded_bytes = match general_purpose::STANDARD.decode(canvas_data_b64) {
            Ok(b) => b,
            Err(_) => return Ok(response.image),
        };
        let json_str = String::from_utf8(decoded_bytes).unwrap_or_default();

        // 🔑 暴力堅固的數字萃取器：自動把 "720}" 變成 720.0
        let extract_f32 = |s: &str| -> f32 {
            let clean: String = s.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
            clean.parse().unwrap_or(0.0)
        };

        // 讀取寬高
        let w_part = json_str.split("\"width\":").nth(1).unwrap_or("0");
        let width = extract_f32(w_part.split(',').next().unwrap_or("0"));

        let h_part = json_str.split("\"height\":").nth(1).unwrap_or("0");
        let height = extract_f32(h_part.split(',').next().unwrap_or("0").split('}').next().unwrap_or("0"));

        if width == 0.0 || height == 0.0 {
            return Ok(response.image);
        }

        // 建立畫布
        let mut canvas = Canvas::new(width, height);

        // 🔑 暴力抓取所有陣列座標：無視任何格式，直接把數字通通抓出來排好
        let args_str = json_str.split("\"args\":").nth(1).unwrap_or("");
        let args_array_str = args_str.split("],\"").next().unwrap_or(args_str);
        
        let clean_args = args_array_str.replace('[', "").replace(']', "");
        let nums: Vec<f32> = clean_args
            .split(',')
            .filter_map(|s| s.trim().parse::<f32>().ok())
            .collect();

        // 每次取 8 個數字一組 (來源 x,y,w,h + 目標 x,y,w,h) 來拼圖
        for chunk in nums.chunks(8) {
            if chunk.len() == 8 {
                let src_rect = Rect::new(chunk[0], chunk[1], chunk[2], chunk[3]);
                let des_rect = Rect::new(chunk[4], chunk[5], chunk[6], chunk[7]);
                canvas.copy_image(&response.image, src_rect, des_rect);
            }
        }

        Ok(canvas.get_image())
    }
}

register_source!(Jmtt, DeepLinkHandler, BaseUrlProvider, PageImageProcessor);

#[cfg(test)]
mod test;