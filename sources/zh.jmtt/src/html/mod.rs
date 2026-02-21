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

use crate::{ image::{ extract_js_config, get_pieces_num }, url::Url };

pub trait GenManga {
    fn list(&self) -> Result<MangaPageResult>;
    fn detail(&self, manga: &mut Manga) -> Result<()>;
    fn chapters(&self) -> Result<Vec<Chapter>>;
    fn chapter(&self, chapter_key: &str) -> Result<Vec<Page>>;
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

            let img_node = item.select_first("img.lazy_img").ok_or_else(|| error!("No cover found"))?;

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
            .map_or(false, |tags| { tags.iter().any(|t| t == "完結" || t == "完结") });

        manga.status = if is_completed { MangaStatus::Completed } else { MangaStatus::Ongoing };

        let is_webtoon = manga.tags
            .as_ref()
            .map_or(false, |tags| { tags.iter().any(|t| t == "韓漫" || t == "韩漫") });

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

    fn chapter(&self, chapter_key: &str) -> Result<Vec<Page>> {
        let mut pages: Vec<Page> = Vec::new();
        let mut aid = chapter_key.to_string();

        // 嘗試從頁面 JS 的 window.infiniteScrollConfig 取得當前章節的 aid 作為雙重保險
        // URL 上面也有比對的方法 https://18comic.vip/album/<禁漫車號不需要開頭JM>
        // ex: https://18comic.vip/album/1216233
        // 禁漫車號不需要開頭JM ex: JM1216233 -> 1216233
        if
            let Some(script_aid) = self
                .select("script")
                .into_iter()
                .flatten()
                .find_map(|node| {
                    let text = node.text().unwrap_or_default();
                    if text.contains("infiniteScrollConfig") {
                        extract_js_config(&text, "currentAid").map(|s| s.to_string())
                    } else {
                        None
                    }
                })
        {
            if !script_aid.is_empty() {
                aid = script_aid;
            }
        }

        let items = self
            .select("div.center.scramble-page.spnotice_chk > img")
            .ok_or_else(|| error!("No chapter list found"))?;

        for item in items {
            let original_url = item.attr("data-original").unwrap_or_default().trim().to_string();
            // 去掉 URL 的 query string（? 之後的部分）
            let original_url = original_url.split('?').next().unwrap_or(&original_url).to_string();

            // 判斷是否為 WebP 混淆圖片，若是則計算切片數並透過 PageContext 傳遞
            if original_url.contains(".webp") {
                // JS 傳給 get_num 的 t (parentId) 為純檔案名稱數字，例如：
                // e.id: "album_photo_00005.webp" or "00005.webp"
                // parentId (t): "00005"
                let mut img_id = item.attr("id").unwrap_or_default().trim().to_string();

                // 1. 去掉 .webp 副檔名
                if let Some(pos) = img_id.find('.') {
                    img_id.truncate(pos);
                }

                // 2. 去掉 album_photo_ 或類似的前綴字串，只留數字
                if let Some(last_index) = img_id.rfind('_') {
                    img_id = img_id[last_index + 1..].to_string();
                }

                // 取得圖片分塊數
                // ex: aid => 1216233, img_id => 00005
                let pieces = get_pieces_num(&aid, &img_id);

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
