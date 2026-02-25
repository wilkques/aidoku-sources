use aidoku::{
    Chapter, Manga, MangaPageResult, MangaStatus, Page, PageContent, Result, Viewer,
    alloc::{String, Vec, string::ToString as _},
    imports::html::Document,
    prelude::*,
};

use crate::{fetch::Fetch, js_packer, settings, url::Url};

pub trait GenManga {
    fn list(&self) -> Result<MangaPageResult>;
    fn detail(&self, manga: &mut Manga) -> Result<()>;
    fn chapters(&self) -> Result<Vec<Chapter>>;
    fn chapter(url: String, body: String) -> Result<Vec<Page>>;
}

impl GenManga for Document {
    fn list(&self) -> Result<MangaPageResult> {
        let mut mangas: Vec<Manga> = Vec::new();

        let items = self
            .select("ul.mh-list > li")
            .ok_or_else(|| error!("No manga items found"))?;

        for item in items {
            let html_a_tag = item
                .select_first("a")
                .ok_or_else(|| error!("No link found"))?;

            let id = html_a_tag
                .attr("href")
                .ok_or_else(|| error!("No href found"))?
                .trim_matches('/')
                .to_string();

            let title = html_a_tag
                .attr("title")
                .ok_or_else(|| error!("No title found"))?
                .trim()
                .to_string();

            let url = Url::book(id.clone())?.to_string();

            let cover = item
                .select_first("p.mh-cover")
                .ok_or_else(|| error!("No cover found"))?
                .attr("style")
                .ok_or_else(|| error!("No style found"))?
                .replace("background-image: url(", "")
                .replace(")", "");

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
            .select_first("div.banner_detail_form > div.cover > img")
            .ok_or_else(|| error!("No cover found"))?
            .attr("src");

        manga.title = self
            .select_first("div.banner_detail_form > div.info > p.title")
            .ok_or_else(|| error!("No title found"))?
            .own_text()
            .ok_or_else(|| error!("No title found"))?
            .trim()
            .to_string();

        manga.authors = Some(
            self.select_first("div.banner_detail_form > div.info > p.subtitle > a")
                .ok_or_else(|| error!("No authors found"))?
                .text()
                .ok_or_else(|| error!("No authors found"))?
                .trim()
                .split(" ")
                .map(|a| a.to_string())
                .collect::<Vec<String>>(),
        );

        manga.artists = Some(Vec::new());

        manga.description = self
            .select(".banner_detail_form>.info>.content")
            .map(|list| list.text().unwrap_or_default().trim().to_string());

        manga.tags = self
            .select(".banner_detail_form>.info>.tip>span.block:nth-child(2)>a")
            .map(|list| {
                list.map(|element| {
                    element
                        .select_first("span")
                        .unwrap()
                        .text()
                        .unwrap_or_default()
                        .trim()
                        .to_string()
                })
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>()
            });

        manga.status = match self
            .select(".banner_detail_form>.info>.tip>span.block:nth-child(1)>span")
            .map(|list| list.text().unwrap_or_default())
            .unwrap_or_default()
            .trim()
        {
            "连载中" => MangaStatus::Ongoing,
            "已完结" => MangaStatus::Completed,
            _ => MangaStatus::Unknown,
        };

        Ok(())
    }

    fn chapters(&self) -> Result<Vec<Chapter>> {
        let mut chapters: Vec<Chapter> = Vec::new();

        let items = self
            .select(".detail-list-select>li,.detail-list-select>.chapteritem>li")
            .ok_or_else(|| error!("No chapter items found"))?;

        for item in items {
            let html_a_tag = item
                .select_first("a")
                .ok_or_else(|| error!("No link found"))?;

            let key = html_a_tag
                .attr("href")
                .ok_or_else(|| error!("No href found"))?
                .trim_matches('/')
                .to_string();

            let url = Url::chapter(key.clone())?.to_string();

            let mut title_str = html_a_tag
                .own_text()
                .unwrap_or_else(|| {
                    html_a_tag
                        .select_first(".info > .title")
                        .unwrap()
                        .own_text()
                        .unwrap_or_default()
                })
                .trim()
                .to_string();

            if html_a_tag.select_first(".info > .detail-lock").is_some() {
                title_str.push_str(" --鎖章節--");
            }

            let title = Some(title_str);

            chapters.push(Chapter {
                key,
                title,
                url: Some(url),
                ..Default::default()
            });
        }

        Ok(chapters)
    }

    fn chapter(url: String, body: String) -> Result<Vec<Page>> {
        // Extract DM5 variables from the chapter page
        let cid =
            js_packer::extract_dm5_var(&body, "DM5_CID").ok_or_else(|| error!("No DM5_CID"))?;
        let image_count: usize = js_packer::extract_dm5_var(&body, "DM5_IMAGE_COUNT")
            .ok_or_else(|| error!("No DM5_IMAGE_COUNT"))?
            .parse()
            .map_err(|_| error!("Bad IMAGE_COUNT"))?;
        let mid =
            js_packer::extract_dm5_var(&body, "DM5_MID").ok_or_else(|| error!("No DM5_MID"))?;
        let viewsign = js_packer::extract_dm5_var(&body, "DM5_VIEWSIGN")
            .ok_or_else(|| error!("No DM5_VIEWSIGN"))?;
        let viewsign_dt = js_packer::extract_dm5_var(&body, "DM5_VIEWSIGN_DT")
            .ok_or_else(|| error!("No DM5_VIEWSIGN_DT"))?;
        let dm5_key = js_packer::extract_dm5_key(&body).unwrap_or("");

        let base_url = settings::get_base_url();
        let mut pages: Vec<Page> = Vec::new();
        let mut api_page = 1;

        while pages.len() < image_count {
            let api_url = format!(
                "{}/chapterfun.ashx?cid={}&page={}&key={}&language=1&gtk=6&_cid={}&_mid={}&_dt={}&_sign={}",
                base_url, cid, api_page, dm5_key, cid, mid, viewsign_dt, viewsign
            );

            let packed = Fetch::get(api_url)?.header("Referer", &url).string()?;

            let decoded =
                js_packer::unpack(&packed).ok_or_else(|| error!("Failed to unpack JS"))?;

            let urls = js_packer::extract_image_urls(&decoded)
                .ok_or_else(|| error!("Failed to extract image URLs"))?;

            if urls.is_empty() {
                break;
            }

            for img_url in &urls {
                pages.push(Page {
                    content: PageContent::url(img_url.clone()),
                    ..Default::default()
                });
            }

            api_page += urls.len();
        }

        Ok(pages)
    }
}
