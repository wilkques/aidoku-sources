use aidoku::{
    Chapter, Manga, MangaPageResult, MangaStatus, Page, PageContent, Result, Viewer, alloc::{String, Vec, string::ToString as _}, imports::html::Document, prelude::*
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
            .select(".mh-item")
            .ok_or_else(|| error!("No manga items found"))?;

        for item in items {
            let id = item
                .select_first("a")
                .ok_or_else(|| error!("No link found"))?
                .attr("href")
                .ok_or_else(|| error!("No link found"))?
                .split("/")
                .last()
                .unwrap_or_default()
                .to_string();

            let url = Url::book(id.clone())?.to_string();

            let cover = item
                .select_first("a>p")
                .ok_or_else(|| error!("No cover found"))?
                .attr("style")
                .ok_or_else(|| error!("No style found"))?
                .replace("background-image: url(", "")
                .replace(")", "");

            let title = item
                .select_first(".mh-item-detali>h2>a")
                .ok_or_else(|| error!("No title found"))?
                .text()
                .ok_or_else(|| error!("No title found"))?
                .trim()
                .to_string();

            let viewer = Viewer::Webtoon;

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
        manga.cover = self
            .select_first(".banner_detail_form>.cover>img")
            .ok_or_else(|| error!("No cover found"))?
            .attr("src");

        manga.title = self
            .select_first(".banner_detail_form>.info>h1")
            .ok_or_else(|| error!("No title found"))?
            .text()
            .ok_or_else(|| error!("No title found"))?
            .trim()
            .to_string();

        manga.authors = self
            .select(".banner_detail_form>.info>p:nth-child(3)")
            .map(|list| {
                list.text()
                    .unwrap_or_default()
                    .trim()
                    .replace("作者：", "")
                    .split("&")
                    .map(|a| a.to_string())
                    .collect::<Vec<String>>()
            });

        manga.artists = Some(Vec::new());

        manga.description = self
            .select(".banner_detail_form>.info>.content")
            .map(|list| list.text().unwrap_or_default().trim().to_string());

        manga.tags = self
            .select(".banner_detail_form>.info>p:nth-child(5)>span>a")
            .map(|list| {
                list.map(|element| element.text().unwrap_or_default().trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<String>>()
            });

        manga.status = match self
            .select(".banner_detail_form>.info>p:nth-child(4)>span:nth-child(1)>span")
            .map(|list| list.text().unwrap_or_default())
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
            .select("#detail-list-select>li>a")
            .ok_or_else(|| error!("No chapter items found"))?;

        for (index, item) in items.enumerate() {
            let href = item.attr("href").unwrap_or_default();

            if href.is_empty() {
                continue;
            }

            let key = href.split("/").last().unwrap_or_default().to_string();

            let url = Url::chapter(key.clone())?.to_string();

            let title = Some(item.text().unwrap_or_default().trim().to_string());

            let chapter_number = Some((index + 1) as f32);

            chapters.push(Chapter {
                key,
                title,
                chapter_number,
                url: Some(url),
                ..Default::default()
            });
        }

        chapters.reverse();

        Ok(chapters)
    }

    fn chapter(&self) -> Result<Vec<Page>> {
        let mut pages: Vec<Page> = Vec::new();

        let items = self
            .select(".comicpage>div>img,#cp_img>img")
            .ok_or_else(|| error!("No chapter img found"))?;

        for item in items {
            let href = item.attr("data-original").unwrap_or_default();

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
