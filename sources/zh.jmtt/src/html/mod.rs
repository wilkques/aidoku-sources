use aidoku::{
    Chapter,
    HashMap,
    Manga,
    MangaPageResult,
    MangaStatus,
    Page,
    PageContent,
    PageContext,
    Result,
    Viewer,
    alloc::{ String, Vec, string::ToString as _ },
    imports::html::Document,
    prelude::*,
};

use crate::{ image::{ clean_img_filename, extract_js_config, get_pieces_num }, url::Url };

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

            let img_node = item.select_first("a > img").ok_or_else(|| error!("No cover found"))?;

            let cover = img_node
                .attr("data-original")
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
                list.map(|element|
                    element.text().unwrap_or_default().trim().to_string()
                ).collect::<Vec<String>>()
            });

        manga.artists = Some(Vec::new());

        manga.description = self
            .select("h2.p-t-5.p-b-5")
            .map(|list| list.text().unwrap_or_default().replace("敘述：", "").trim().to_string());

        manga.tags = self
            .select("span[itemprop='genre'][data-type='tags'] a.web-tags-tag")
            .map(|list| {
                list.map(|element|
                    element.text().unwrap_or_default().trim().to_string()
                ).collect::<Vec<String>>()
            });

        let is_completed = manga.tags
            .as_ref()
            .map_or(false, |tags| { tags.iter().any(|t| (t == "完結" || t == "完结")) });

        manga.status = if is_completed { MangaStatus::Completed } else { MangaStatus::Ongoing };

        let is_webtoon = manga.tags
            .as_ref()
            .map_or(false, |tags| { tags.iter().any(|t| (t == "韓漫" || t == "韩漫")) });

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
                item
                    .select_first("h3.h2_series")
                    .ok_or_else(|| error!("No chapter items found"))?
                    .text()
                    .unwrap_or_default()
                    .split_whitespace()
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

        // 從頁面 JS 的 infiniteScrollConfig 中取得下一話的 aid
        let aid = self
            .select("script")
            .into_iter()
            .flatten()
            .find_map(|node| {
                let text = node.text().unwrap_or_default();
                if text.contains("infiniteScrollConfig") {
                    extract_js_config(&text, "nextChapterAid").map(|s| s.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let items = self
            .select("div.center.scramble-page.spnotice_chk > img")
            .ok_or_else(|| error!("No chapter list found"))?;

        for item in items {
            let original_url = item.attr("data-original").unwrap_or_default().trim().to_string();

            // 判斷是否為 WebP 混淆圖片，若是則計算切片數並透過 PageContext 傳遞
            if original_url.contains(".webp") {
                let raw_filename = original_url.split('/').last().unwrap_or("");
                let clean_name = clean_img_filename(raw_filename);
                let pieces = get_pieces_num(&aid, &clean_name);

                // 透過 PageContext (HashMap) 傳遞 pieces，不污染圖片 URL
                let mut ctx: PageContext = HashMap::new();
                ctx.insert("pieces".to_string(), pieces.to_string());

                pages.push(Page {
                    content: PageContent::url_context(original_url, ctx),
                    ..Default::default()
                });
            } else {
                pages.push(Page {
                    content: PageContent::url(original_url),
                    ..Default::default()
                });
            }
        }

        Ok(pages)
    }
}
