use aidoku::{
    Chapter, Manga, MangaPageResult, Page, PageContent, Result, Viewer,
    alloc::{String, Vec},
    prelude::format,
};
use serde::Deserialize;

use crate::{crypto, fetch::Fetch, settings, url::Url};

// ── 閱讀 API ─────────────────────────────────────────────────
// GET /v2.0/apis/manga/reading?code={code}&cid={cid}&v=v4.300102

#[derive(Deserialize, Debug)]
pub struct ReadingApiResponse {
    pub data: ReadingData,
}

#[derive(Deserialize, Debug)]
pub struct ReadingData {
    pub scans: ScansValue,
}

#[derive(Deserialize, Debug)]
pub struct FaileImgResponse {
    pub data: Option<String>,
}

/// scans 欄位可能是陣列（明文），也可能是加密字串（isEn=true 時）
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum ScansValue {
    Array(Vec<ScanItem>),
    Encoded(String),
}

#[derive(Deserialize, Debug)]
pub struct ScanItem {
    pub url: String,
}

// ── 漫畫列表 API ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ApiResponse {
    pub data: ApiData,
}

#[derive(Deserialize)]
pub struct ApiData {
    pub items: Vec<ApiItem>,
    #[serde(rename = "isEnd")]
    pub is_end: bool,
}

#[derive(Deserialize)]
pub struct ApiItem {
    pub name: String,
    pub manga_code: String,
    pub cover: Option<String>,
}

// ── 章節列表 API ─────────────────────────────────────────────
// /v2.0/apis/manga/chapterByPage?code={key}&page={page}

#[derive(Deserialize)]
pub struct ChapterApiResponse {
    pub data: ChapterApiData,
}

#[derive(Deserialize)]
pub struct ChapterApiData {
    pub items: Vec<ChapterApiItem>,
    #[serde(rename = "isEnd")]
    pub is_end: i32,
}

#[derive(Deserialize)]
pub struct ChapterApiItem {
    pub id: i64,
    #[serde(rename = "chapterName")]
    pub chapter_name: Option<String>,
    pub order: Option<f32>,
}

pub struct GenManga;

impl GenManga {
    pub fn list(resp: ApiResponse) -> Result<MangaPageResult> {
        let has_next_page = !resp.data.is_end;
        let entries = resp
            .data
            .items
            .into_iter()
            .map(|item| Manga {
                key: item.manga_code,
                cover: item.cover,
                title: item.name,
                viewer: Viewer::Webtoon,
                ..Default::default()
            })
            .collect();

        Ok(MangaPageResult {
            entries,
            has_next_page,
        })
    }

    pub fn chapters(key: &str) -> Result<Vec<Chapter>> {
        let mut chapters: Vec<Chapter> = Vec::new();
        let mut page = 1;

        loop {
            let url = Url::book(String::from(key), page).to_string();
            let data: ChapterApiResponse = Fetch::get(url)?
            .header("Referer", &format!("{}/v2.0/apis/manga/chapterByPage", settings::get_base_url()))
            .json_owned()?;

            for item in data.data.items {
                chapters.push(Chapter {
                    key: format!("{}", item.id),
                    title: item.chapter_name,
                    chapter_number: item.order,
                    ..Default::default()
                });
            }

            if data.data.is_end != 0 {
                break;
            }
            page += 1;
        }

        Ok(chapters)
    }

    pub fn chapter(resp: ReadingApiResponse, manga_key: &str, chapter_key: &str) -> Result<Vec<Page>> {
        let scans: Vec<ScanItem> = match resp.data.scans {
            ScansValue::Array(items) => items,
            ScansValue::Encoded(enc) => {
                match crypto::decrypt_scans(&enc, "happymh.com") {
                    Some(items) => items.into_iter().map(|s| ScanItem { url: s.url }).collect(),
                    None => {
                        aidoku::println!("[happy] chapter: decrypt failed -> entering fallback");
                        return Self::chapter_fallback(manga_key, chapter_key);
                    }
                }
            }
        };

        Ok(scans.into_iter().map(|s| Self::make_page(s.url)).collect())
    }

    fn chapter_fallback(manga_key: &str, chapter_key: &str) -> Result<Vec<Page>> {
        let base_url = settings::get_base_url();
        let referer = format!("{}/mangaread/{}/{}", base_url, manga_key, chapter_key);
        let post_url = format!("{}/apis/m/faileImg", base_url);

        let mut pages = Vec::new();
        let mut i = 0;
        loop {
            let body = format!("{{\"index\":{},\"cid\":\"{}\",\"code\":\"{}\"}}", i, chapter_key, manga_key);
            let resp: FaileImgResponse = match Fetch::post(post_url.clone())?
                .header("Referer", &referer)
                .header("X-Requested-With", "XMLHttpRequest")
                .header("Content-Type", "application/json")
                .body(body.as_bytes())
                .json_owned()
            {
                Ok(r) => r,
                Err(_) => break,
            };
            match resp.data {
                Some(u) if !u.is_empty() => {
                    pages.push(Self::make_page(u));
                }
                _ => break,
            }
            i += 1;
        }
        Ok(pages)
    }

    fn make_page(raw_url: String) -> Page {
        let url = if let Some(pos) = raw_url.find("?q=") {
            format!("{}?q=99", &raw_url[..pos])
        } else if raw_url.contains('?') {
            format!("{}&q=99", raw_url)
        } else {
            format!("{}?q=99", raw_url)
        };
        Page {
            content: PageContent::url(url),
            ..Default::default()
        }
    }
}
