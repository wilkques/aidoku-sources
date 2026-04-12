use aidoku::{
    Chapter, Manga, MangaPageResult, MangaStatus, Page, PageContent, Result, Viewer,
    alloc::{String, Vec, string::ToString as _},
    imports::{html::Document, std::current_date},
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
            .select(".chapter-loveYou")
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

            chapters.push(Chapter {
                key,
                title,
                url: Some(url),
                date_uploaded,
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
