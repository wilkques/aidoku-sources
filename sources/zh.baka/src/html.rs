use aidoku::{
    Chapter, Manga, MangaPageResult, MangaStatus, Page, PageContent, Result, Viewer,
    alloc::{String, Vec, string::ToString as _},
    imports::html::Document,
    prelude::*,
};

use crate::url::Url;

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
            .select("#loop-content .c-image-hover")
            .ok_or_else(|| error!("No manga items found"))?;

        for item in items {
            let html_a_tag = item
                .select_first("a")
                .ok_or_else(|| error!("No link found"))?;

            let id = html_a_tag
                .attr("href")
                .ok_or_else(|| error!("No link found"))?
                .trim_matches('/')
                .to_string()
                .split('/')
                .last()
                .unwrap_or_default()
                .to_string();

            let title = html_a_tag
                .attr("title")
                .ok_or_else(|| error!("No link found"))?
                .to_string();

            let url = Url::book(id.clone())?.to_string();

            let cover = html_a_tag
                .select_first("img")
                .ok_or_else(|| error!("No cover found"))?
                .attr("src")
                .ok_or_else(|| error!("No style found"))?
                .to_string();

            let viewer = match item
                .select_first(".img-responsive")
                .ok_or_else(|| error!("No viewer found"))?
                .text()
                .unwrap_or_default()
                .trim()
            {
                "韩漫" => Viewer::Webtoon,
                _ => Viewer::RightToLeft,
            };

            mangas.push(Manga {
                key: id,
                cover: Some(cover),
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
        manga.authors = self.select(".author-content > a").map(|list| {
            list.map(|element| element.text().unwrap_or_default().trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>()
        });

        manga.artists = Some(Vec::new());

        manga.description = Some(
            self.select_first(".post-content_item:last-child > div > p")
                .ok_or_else(|| error!("No description found"))?
                .text()
                .unwrap_or_default()
                .trim()
                .to_string(),
        );

        manga.tags = self.select(".tags-content > a").map(|list| {
            list.map(|element| element.text().unwrap_or_default().trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>()
        });

        manga.status = match self
            .select_first(".post-content_item:nth-last-of-type(2) > .summary-content")
            .ok_or_else(|| error!("No status found"))?
            .text()
            .unwrap_or_default()
            .trim()
        {
            "连载中" => MangaStatus::Ongoing,
            "已完结" => MangaStatus::Completed,
            _ => MangaStatus::Unknown,
        };

        manga.viewer = Viewer::Webtoon;

        Ok(())
    }

    fn chapters(&self) -> Result<Vec<Chapter>> {
        let mut chapters: Vec<Chapter> = Vec::new();

        let items = self
            .select(".chapter-cubical > a")
            .ok_or_else(|| error!("No chapter items found"))?;

        for item in items {
            let href = item.attr("chapter-data-url").unwrap_or_default();

            if href.is_empty() {
                continue;
            }

            let info = href.trim_matches('/').split("/").collect::<Vec<&str>>();

            let title = Some(item.text().unwrap_or_default().trim().to_string());

            let key = info[info.len() - 2..].join("/");

            let url = Url::chapter(key.clone())?.to_string();

            chapters.push(Chapter {
                key,
                title,
                url: Some(url),
                ..Default::default()
            });
        }

        Ok(chapters)
    }

    fn chapter(&self) -> Result<Vec<Page>> {
        let mut pages: Vec<Page> = Vec::new();

        let items = self
            .select("img[id^=image-]")
            .ok_or_else(|| error!("No chapter img found"))?;

        for item in items {
            let href = item.attr("src").unwrap_or_default();

            if href.is_empty() {
                continue;
            }

            let url = href.trim().to_string();

            pages.push(Page {
                content: PageContent::url(url),
                ..Default::default()
            })
        }

        Ok(pages)
    }
}
