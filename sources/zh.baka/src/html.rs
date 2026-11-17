use aidoku::{
    Chapter, Manga, MangaPageResult, MangaStatus, Page, PageContent, Result, Viewer,
    alloc::{String, Vec, string::ToString as _},
    imports::{
        html::{Document, Element},
        std::current_date,
    },
    prelude::*,
};

use crate::{settings, url::Url};

pub trait GenManga {
    fn list(&self) -> Result<MangaPageResult>;
    fn detail(&self, manga: &mut Manga) -> Result<()>;
    fn chapters(&self) -> Result<Vec<Chapter>>;
    fn chapter(&self) -> Result<Vec<Page>>;
}

impl GenManga for Document {
    fn list(&self) -> Result<MangaPageResult> {
        let mut mangas: Vec<Manga> = Vec::new();

        // 站方改版後列表是 `<article class="mkjp-sr__item">`（封面 `.mkjp-sr__cover`、
        // 標題 `.mkjp-sr__title > a`、分類 `.mkjp-sr__genre`）。
        let items = self
            .select("#loop-content .mkjp-sr__item")
            .ok_or_else(|| error!("No manga items found"))?;

        for item in items {
            let html_a_tag = item
                .select_first(".mkjp-sr__title a")
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

            let title = decode_entities(html_a_tag.text().unwrap_or_default().trim());

            let url = Url::book(id.clone())?.to_string();

            let cover = item
                .select_first(".mkjp-sr__cover img")
                .and_then(|img| image_url(&img));

            let viewer = match item
                .select_first(".mkjp-sr__genre")
                .and_then(|genre| genre.text())
                .unwrap_or_default()
                .trim()
            {
                "韩漫" => Viewer::Webtoon,
                _ => Viewer::RightToLeft,
            };

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
        manga.authors = self.select(".author-content > a").map(|list| {
            list.map(|element| element.text().unwrap_or_default().trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>()
        });

        manga.artists = Some(Vec::new());

        manga.description = Some(
            self.select_first(".mkjp-summary__text")
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

        // 狀態列沒有固定位置（前後會多出收藏、評論等項目），改用標題文字「状态」找
        let status_text = self
            .select(".post-content_item")
            .and_then(|items| {
                items
                    .filter(|item| {
                        item.select_first(".summary-heading")
                            .and_then(|heading| heading.text())
                            .is_some_and(|text| text.contains("状态"))
                    })
                    .find_map(|item| item.select_first(".summary-content")?.text())
            })
            .unwrap_or_default();

        manga.status = match status_text.trim() {
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
            .select(".mkjp-chapter")
            .ok_or_else(|| error!("No chapter items found"))?;

        for item in items {
            let atag = item
                .select_first("a")
                .ok_or_else(|| error!("No link found"))?;

            let href = atag.attr("href").unwrap_or_default();

            if href.is_empty() {
                continue;
            }

            let info = href.trim_matches('/').split("/").collect::<Vec<&str>>();

            let key = info[info.len() - 2..].join("/");

            let title = Some(atag.text().unwrap_or_default());

            let url = Url::chapter(key.clone())?.to_string();

            // Parse Chinese date strings into Unix timestamps.
            // Handles:
            //   Absolute: "YYYY 年 M 月 D 日"
            //   Relative: "X 天 前" / "X 周 前" / "X 小时 前" / "X 分钟 前"
            let date_uploaded = item
                .select_first(".chapter-release-date i")
                .and_then(|el| el.text())
                .and_then(|text| {
                    let text = text.trim().to_string();
                    let now = current_date();

                    if text.contains('年') {
                        // Absolute: "2026 年 1 月 11 日"
                        let s = text
                            .replace("年", "")
                            .replace("月", "")
                            .replace("日", "");
                        let parts: Vec<&str> = s.split_whitespace().collect();
                        if parts.len() >= 3 {
                            let year = parts[0].parse::<i64>().ok()?;
                            let month = parts[1].parse::<i64>().ok()?;
                            let day = parts[2].parse::<i64>().ok()?;
                            // Days since Unix epoch via Julian Day Number
                            let a = (14 - month) / 12;
                            let y = year + 4800 - a;
                            let m = month + 12 * a - 3;
                            let jdn = day + (153 * m + 2) / 5 + 365 * y
                                + y / 4 - y / 100 + y / 400 - 32045;
                            // JDN of 1970-01-01 is 2440588
                            Some((jdn - 2440588) * 86400)
                        } else {
                            None
                        }
                    } else {
                        // Relative: "X 天/周/小时/分钟 前"
                        let parts: Vec<&str> = text.split_whitespace().collect();
                        if parts.len() >= 2 {
                            let n = parts[0].parse::<i64>().ok()?;
                            let unit = parts[1];
                            let offset_secs = match unit {
                                "分钟" => n * 60,
                                "小时" => n * 3600,
                                "天"   => n * 86400,
                                "周"   => n * 7 * 86400,
                                _      => return None,
                            };
                            Some(now - offset_secs)
                        } else {
                            None
                        }
                    }
                });

            // 搶先閱讀 / 付費章節會多一個 `.mkjp-ea-badge` 標章，在列表上標成鎖定
            let locked = item.select_first(".mkjp-ea-badge").is_some();

            chapters.push(Chapter {
                key,
                title,
                url: Some(url),
                date_uploaded,
                locked,
                ..Default::default()
            });
        }

        Ok(chapters)
    }

    fn chapter(&self) -> Result<Vec<Page>> {
        // 每頁結構是 `<div class="page-break" data-index="N"><img id="image-N" data-src=...>`，
        // 後面再跟一個 `<noscript><img src=... class="mkjp-noscript"></noscript>` 備用圖（同一張）。
        // 只抓有 `id="image-N"` 的那張，noscript 的沒有 id 會自動排除，否則每頁都會出現兩次。
        // 網址屬性舊版是 `data-manga-src`、現在是 `data-src`，`image_url` 兩種都找。
        let mut items: Vec<(i32, String)> = self
            .select("img[id^=image-]")
            .map(|items| {
                items
                    .filter_map(|item| {
                        let index = item
                            .attr("id")?
                            .trim_start_matches("image-")
                            .parse::<i32>()
                            .unwrap_or(i32::MAX);
                        Some((index, image_url(&item)?))
                    })
                    .collect()
            })
            .unwrap_or_default();

        // DOM 目前本來就依序排列；照 `image-N` 的編號排序，站方哪天打亂 DOM 順序也不受影響
        items.sort_by_key(|(index, _)| *index);

        let mut pages: Vec<Page> = Vec::new();
        let mut seen: Vec<String> = Vec::new();

        for (_, url) in items {
            if seen.contains(&url) {
                continue;
            }
            seen.push(url.clone());

            pages.push(Page {
                content: PageContent::url(url),
                ..Default::default()
            });
        }

        // 限登入的章節，未登入時頁面上不會有任何 `image-N`；與其回傳空白章節，不如講清楚
        if pages.is_empty() {
            bail!(
                "這個章節沒有圖片，可能需要登入才能看。請到 瀏覽 → 巴卡漫畫 → 齒輪 →「登入帳號（{}）」登入後再試",
                settings::cf_host(&settings::get_base_url())
            );
        }

        Ok(pages)
    }
}

// WordPress（Madara 主題）常用 lazy-load：`src` 只是 `data:` 佔位圖，真正的網址放在
// `data-src`／`data-lazy-src`／`srcset`（閱讀頁舊版是 `data-manga-src`）。依序找第一個
// 像樣的 http(s) 網址。封面跟章節圖片共用。
fn image_url(img: &Element) -> Option<String> {
    let srcset_first = img.attr("data-srcset").or_else(|| img.attr("srcset")).and_then(|srcset| {
        srcset
            .split(',')
            .next()
            .and_then(|entry| entry.split_whitespace().next())
            .map(|url| url.to_string())
    });

    ["data-manga-src", "data-src", "data-lazy-src", "data-original"]
        .iter()
        .filter_map(|name| img.attr(name))
        .chain(srcset_first)
        .chain(img.attr("src"))
        .map(|url| url.trim().to_string())
        .find(|url| url.starts_with("http") || url.starts_with("//"))
}

fn decode_entities(text: &str) -> String {
    // `&amp;` 放最後，才不會把 `&amp;lt;` 這種字面文字多解一層
    text.replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}
