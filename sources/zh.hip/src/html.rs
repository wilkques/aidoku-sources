use aidoku::{
    Chapter, Manga, MangaPageResult, MangaStatus, Page, PageContent, Result, Viewer,
    alloc::{String, Vec, string::ToString as _},
    imports::{html::Document, std::parse_date},
    prelude::*,
};
use serde::Deserialize;

use crate::{
    decoder::{ChapterImagesResponse, decode_chapter_images},
    fetch::Fetch,
    settings,
    url::Url,
};

// 章節列表不在 detail 頁的靜態 HTML 裡，是前端另外打 API 拿的
// GET {api_url}/v1/manga/chapters?mid={mid}&page={page}&per_page={per_page}&order=asc
#[derive(Deserialize)]
struct ChaptersApiResponse {
    data: ChaptersApiData,
}

#[derive(Deserialize)]
struct ChaptersApiData {
    total_pages: i32,
    items: Vec<ChapterApiItem>,
}

#[derive(Deserialize)]
struct ChapterApiItem {
    hid: String,
    chapter_number: f32,
    title: String,
    updated_at: String,
}

pub trait GenManga {
    fn list(&self) -> Result<MangaPageResult>;
    fn detail(&self, manga: &mut Manga) -> Result<()>;
    fn chapters(&self) -> Result<Vec<Chapter>>;
    fn chapter(&self) -> Result<Vec<Page>>;
}

impl GenManga for Document {
    fn list(&self) -> Result<MangaPageResult> {
        let mut mangas: Vec<Manga> = Vec::new();

        let items = self
            .select(".manga-card-link")
            .ok_or_else(|| error!("No manga items found"))?;

        for item in items {
            let id = item
                .attr("href")
                .ok_or_else(|| error!("No link found"))?
                .split("/")
                .last()
                .unwrap_or_default()
                .to_string();

            if id.is_empty() {
                continue;
            }

            let url = Url::book(id.clone())?.to_string();

            let cover = item.select_first("img").and_then(|img| img.attr("src"));

            let title = item
                .select_first(".manga-card-title")
                .and_then(|el| el.text())
                .or_else(|| item.attr("aria-label"))
                .unwrap_or_default()
                .trim()
                .to_string();

            let viewer = Viewer::Webtoon;

            mangas.push(Manga {
                key: id,
                cover,
                title,
                url: Some(url),
                viewer,
                ..Default::default()
            });
        }

        Ok(MangaPageResult {
            entries: mangas.clone(),
            has_next_page: !mangas.is_empty(),
        })
    }

    fn detail(&self, manga: &mut Manga) -> Result<()> {
        let root = self
            .select_first("[data-manga-title]")
            .ok_or_else(|| error!("No manga info found"))?;

        manga.cover = root.attr("data-cover-url");

        manga.title = root
            .attr("data-manga-title")
            .ok_or_else(|| error!("No title found"))?
            .trim()
            .to_string();

        manga.authors = self.select("a[href^=\"/author/\"]").map(|list| {
            list.map(|el| el.text().unwrap_or_default().trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>()
        });

        manga.artists = Some(Vec::new());

        manga.description = self
            .select_first(".whitespace-pre-line")
            .and_then(|el| el.text())
            .map(|text| text.trim().to_string());

        manga.tags = self.select("a[href^=\"/genre/\"]").map(|list| {
            list.map(|el| el.text().unwrap_or_default().trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>()
        });

        manga.status = if self.select_first("a[title=\"連載中\"]").is_some() {
            MangaStatus::Ongoing
        } else if self.select_first("a[title=\"已完結\"]").is_some() {
            MangaStatus::Completed
        } else {
            MangaStatus::Unknown
        };

        manga.viewer = Viewer::Webtoon;

        Ok(())
    }

    fn chapters(&self) -> Result<Vec<Chapter>> {
        // 章節不在 detail 頁的靜態 HTML 裡，前端是另外打 API 拿的
        // (見 https://m.hipmh.com/assets/chapters-manager.*.js 裡的 fetchChapters)
        let mid = self
            .select_first("#chapters-config")
            .ok_or_else(|| error!("No chapters config found"))?
            .attr("data-mid")
            .ok_or_else(|| error!("No mid found"))?;

        let mut chapters: Vec<Chapter> = Vec::new();
        let mut page = 1;

        loop {
            let url = format!(
                "{}/v1/manga/chapters?mid={}&page={}&per_page=50&order=asc",
                settings::get_api_url(),
                mid,
                page
            );

            let data: ChaptersApiResponse = Fetch::get(url)?.json_owned()?;

            if data.data.items.is_empty() {
                break;
            }

            for item in data.data.items {
                let url = Url::chapter(item.hid.clone())?.to_string();

                // "2026-09-10T19:08:04.755913Z" -> 只取日期部分
                let date_uploaded = item
                    .updated_at
                    .split('T')
                    .next()
                    .and_then(|s| parse_date(s, "yyyy-MM-dd"));

                chapters.push(Chapter {
                    key: item.hid,
                    title: Some(item.title),
                    chapter_number: Some(item.chapter_number),
                    date_uploaded,
                    url: Some(url),
                    ..Default::default()
                });
            }

            if page >= data.data.total_pages {
                break;
            }

            page += 1;
        }

        Ok(chapters)
    }

    fn chapter(&self) -> Result<Vec<Page>> {
        // 閱讀頁本身不含明文圖片網址，圖片清單是另外打 API 拿加密過的字串
        let config = self
            .select_first("#chapcontent")
            .ok_or_else(|| error!("No chapter config found"))?;

        let api_hid = config
            .attr("data-api-hid")
            .ok_or_else(|| error!("No api hid found"))?;

        let img_base = config
            .attr("data-chapter-img-base")
            .ok_or_else(|| error!("No chapter img base found"))?;

        let url = format!("{}/v2/chapter?hid={}", settings::get_api_url(), api_hid);

        let data: ChapterImagesResponse = Fetch::get(url)?.json_owned()?;

        let paths = decode_chapter_images(&data.data.images)
            .ok_or_else(|| error!("Failed to decode chapter images"))?;

        let pages = paths
            .into_iter()
            .map(|path| Page {
                content: PageContent::url(format!("{}{}", img_base, path)),
                ..Default::default()
            })
            .collect();

        Ok(pages)
    }
}
