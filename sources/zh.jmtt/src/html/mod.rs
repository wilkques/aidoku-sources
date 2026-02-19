use aidoku::{
    Chapter, 
    Manga, MangaPageResult, MangaStatus, Page, PageContent, Result, Viewer, 
    alloc::{String, Vec, string::ToString as _}, 
    imports::html::Document, 
    prelude::*
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
            .select("div.list-col > div.p-b-15")
            .ok_or_else(|| error!("No manga items found"))?;

        for item in items {
            let id = item
                .select_first("a")
                .ok_or_else(|| error!("No link found"))?
                .attr("href")
                .ok_or_else(|| error!("No link found"))?
                .split("/")
                .nth(2)
                .unwrap_or_default()
                .to_string();

            let url = Url::book(id.clone())?.to_string();

            let img_node = item
                .select_first("a > img")
                .ok_or_else(|| error!("No cover found"))?;

            let cover = img_node.attr("data-original")
                .or_else(|| img_node.attr("src"))
                .ok_or_else(|| error!("No cover attribute found"))?
                .trim()
                .to_string();

            let title = item
                .select_first(".title-truncate")
                .ok_or_else(|| error!("No title found"))?
                .text()
                .ok_or_else(|| error!("No title found"))?
                .trim()
                .to_string();

            mangas.push(Manga {
                key: id,
                cover: Some(cover),
                title,
                url: Some(url),
                ..Default::default()
            });
        }

        Ok(MangaPageResult {
            entries: mangas.clone(),
            has_next_page: !mangas.is_empty(),
        })
    }
    
    fn detail(&self, manga: &mut Manga) -> Result<()> {
        manga.authors = self
            .select("span[itemprop='author'][data-type='author'] a.web-author-tag")
            .map(|list| {
                list.map(|element| element.text().unwrap_or_default().trim().to_string())
                    .collect::<Vec<String>>()
            });

        manga.artists = Some(Vec::new());

        manga.description = self
            .select("h2.p-t-5.p-b-5")
            .map(|list| list.text().unwrap_or_default().replace("敘述：", "").trim().to_string());
            
        manga.tags = self
            .select("span[itemprop='genre'][data-type='tags'] a.web-tags-tag")
            .map(|list| {
                list.map(|element| element.text().unwrap_or_default().trim().to_string())
                    .collect::<Vec<String>>()
            });

        let is_completed = manga.tags.as_ref().map_or(false, |tags| {
            tags.iter().any(|t| t == "完結" || t == "完结")
        });

        manga.status = if is_completed { MangaStatus::Completed } else { MangaStatus::Ongoing };

        let is_webtoon = manga.tags.as_ref().map_or(false, |tags| {
            tags.iter().any(|t| t == "韓漫" || t == "韩漫")
        });

        manga.viewer = if is_webtoon { Viewer::Webtoon } else { Viewer::LeftToRight };

        Ok(())
    }

    fn chapters(&self) -> Result<Vec<Chapter>> {
        let mut chapters: Vec<Chapter> = Vec::new();

        let items = self
            .select("div.episode > ul > a")
            .ok_or_else(|| error!("No chapter items found"))?;

        for (index, item) in items.enumerate() {
            let href = item.attr("href").unwrap_or_default();

            if href.is_empty() {
                continue;
            }

            let key = item.attr("data-album").unwrap_or_default().trim().to_string();

            let url = Url::chapter(key.clone())?.to_string();

            let title = Some(
                item.select_first("h3.h2_series")
                .ok_or_else(|| error!("No chapter items found"))?
                .text()
                .unwrap_or_default().split_whitespace()
                .next()
                .unwrap_or_default()
                .to_string()
            );

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
            .select("div.center.scramble-page.spnotice_chk")
            .ok_or_else(|| error!("No chapter list found"))?;

        for item in items {
            let img_node = match item.select_first("img") {
                Some(node) => node,
                None => continue,
            };

            let mut url = img_node.attr("data-original").unwrap_or_default().trim().to_string();
            if url.is_empty() || url.contains("blank.jpg") {
                url = img_node.attr("src").unwrap_or_default().trim().to_string();
            }
            if url.is_empty() || url.contains("blank.jpg") {
                continue;
            }

            let clean_url = url.split('?').next().unwrap_or(&url).to_string();

            let canvas_data = item
                .select_first("canvas")
                .map(|n| n.attr("data").unwrap_or_default().trim().to_string())
                .unwrap_or_default();

            let content = if canvas_data.is_empty() {
                PageContent::url(clean_url)
            } else {
                let mut ctx = aidoku::HashMap::new();
                ctx.insert(String::from("canvas_data"), canvas_data);
                PageContent::url_context(clean_url, ctx)
            };

            pages.push(Page {
                content,
                ..Default::default()
            });
        }

        Ok(pages)
    }
}
