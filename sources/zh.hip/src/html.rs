use aidoku::{
    Chapter, Manga, MangaPageResult, MangaStatus, Page, PageContent, Result, Viewer,
    alloc::{String, Vec, string::ToString as _},
    imports::{html::Document, net::Request, std::parse_date},
    prelude::*,
};

use crate::{
    decoder::{ChapterImagesResponse, decode_chapter_images, page_number_key},
    fetch::Fetch,
    json::ChaptersApiResponse,
    settings,
    url::Url,
};

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
            list.flat_map(|el| {
                el.text()
                    .unwrap_or_default()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect::<Vec<String>>()
            })
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
                "{}/v1/manga/chapters?mid={}&page={}&per_page=50&order=desc",
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

        let paths = resolve_decoy_duplicates(&img_base, paths);

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

// 解碼出來的頁面清單偶爾會在同一個頁碼相鄰出現兩筆，其中一筆是站方塞的誘餌
// （HTTP 200 但 Content-Type 是 image/png 的 1x1 透明圖），另一筆才是真正的 webp
// 內頁。不篩掉的話 aidoku 的 webtoon viewer 會把誘餌也當成一頁塞進去，撐出一大塊
// 黑色空白（見對話截圖：「一人之下」第58話頁碼46 就中招）。
//
// 誘餌在陣列裡排在真圖前面還是後面沒有固定規律（同一本書不同章節都各遇過一次），
// 所以不能只看順序，只能對兩筆都打一次 HEAD 請求，用回應的 Content-Type 判斷。
// 這種相鄰重複很少見（實測 124 頁的章節只出現 1 組），多打的 HEAD 請求數量可以忽略；
// 如果請求失敗判斷不出來，兩筆都保留，不要亂踢真的內頁。
fn resolve_decoy_duplicates(img_base: &str, paths: Vec<String>) -> Vec<String> {
    let keys: Vec<Option<&str>> = paths.iter().map(|p| page_number_key(p)).collect();

    let mut pairs: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;

    while i + 1 < paths.len() {
        if keys[i].is_some() && keys[i] == keys[i + 1] {
            pairs.push((i, i + 1));
            i += 2;
        } else {
            i += 1;
        }
    }

    if pairs.is_empty() {
        return paths;
    }

    let requests: Vec<Request> = pairs
        .iter()
        .flat_map(|&(a, b)| [a, b])
        .filter_map(|idx| Fetch::head(format!("{}{}", img_base, paths[idx])).ok())
        .collect();

    if requests.len() != pairs.len() * 2 {
        return paths;
    }

    let responses = Request::send_all(requests);

    let mut drop_indices: Vec<usize> = Vec::new();

    for (pair_idx, &(a, b)) in pairs.iter().enumerate() {
        let is_png = |idx: usize| {
            responses
                .get(idx)
                .and_then(|r| r.as_ref().ok())
                .and_then(|r| r.get_header("content-type"))
                .is_some_and(|ct| ct.starts_with("image/png"))
        };

        if is_png(pair_idx * 2) {
            drop_indices.push(a);
        } else if is_png(pair_idx * 2 + 1) {
            drop_indices.push(b);
        }
    }

    paths
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !drop_indices.contains(i))
        .map(|(_, path)| path)
        .collect()
}
