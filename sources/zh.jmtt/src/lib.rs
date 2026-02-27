#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod fetch;
mod helpers;
mod home;
mod html;
mod settings;
mod url;

use aidoku::{
    BaseUrlProvider, Chapter, FilterValue, ImageRequestProvider, ImageResponse, Listing,
    ListingProvider, Manga, MangaPageResult, Page, PageContext, PageImageProcessor, Result, Source,
    alloc::{String, Vec, string::ToString as _, vec},
    imports::{canvas::ImageRef, net::Request},
    prelude::*,
};

use crate::{fetch::Fetch, helpers::reload_image, html::GenManga, url::Url};

struct Jmtt;

impl Source for Jmtt {
    fn new() -> Self {
        Self
    }

    fn get_search_manga_list(
        &self,
        query: Option<String>,
        page: i32,
        filters: Vec<FilterValue>,
    ) -> Result<MangaPageResult> {
        let url = Url::filters(query.as_deref(), page, &filters)?.to_string();

        let response = Fetch::get(url)?.html()?;

        GenManga::list(&response)
    }

    fn get_manga_update(
        &self,
        mut manga: Manga,
        needs_details: bool,
        needs_chapters: bool,
    ) -> Result<Manga> {
        let url = Url::book(manga.key.clone())?.to_string();

        let response = Fetch::get(url)?.html()?;

        if needs_details {
            GenManga::detail(&response, &mut manga)?;
        }

        if needs_chapters {
            manga.chapters = Some(GenManga::chapters(&response, &manga)?);
        }

        Ok(manga)
    }

    fn get_page_list(&self, _: Manga, chapter: Chapter) -> Result<Vec<Page>> {
        let url = Url::chapter(chapter.key.clone())?.to_string();
        let response = Fetch::get(url)?.html()?;

        GenManga::chapter(&response, &chapter)
    }
}

impl BaseUrlProvider for Jmtt {
    fn get_base_url(&self) -> Result<String> {
        Ok(settings::get_base_url())
    }
}

impl PageImageProcessor for Jmtt {
    fn process_page_image(
        &self,
        response: ImageResponse,
        context: Option<PageContext>,
    ) -> Result<ImageRef> {
        let pieces: u32 = context
            .and_then(|ctx| ctx.get("pieces").and_then(|v| v.parse().ok()))
            .unwrap_or(0);

        Ok(reload_image(&response.image, pieces))
    }
}

impl ImageRequestProvider for Jmtt {
    fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
        Ok(Fetch::get(url)?)
    }
}

impl ListingProvider for Jmtt {
    fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
        let url = match listing.id.as_str() {
            "dailymanga" => {
                Url::serialization(helpers::get_current_day_of_week().to_string())?.to_string()
            }
            "newmanga" => Url::filters(
                None,
                page,
                &vec![FilterValue::Select {
                    id: "排序".to_string(),
                    value: "mr".to_string(),
                }],
            )?
            .to_string(),
            "jingmanchinesemanga" => Url::filters(
                Some("禁漫汉化组"),
                page,
                &vec![FilterValue::Select {
                    id: "搜索範圍".to_string(),
                    value: "0".to_string(),
                }],
            )?
            .to_string(),
            "hanmanga" => Url::filters(
                None,
                page,
                &vec![FilterValue::Select {
                    id: "類型".to_string(),
                    value: "hanman".to_string(),
                }],
            )?
            .to_string(),
            "recommendmanga" => Url::promotes("29".to_string(), page)?.to_string(),
            "jingmanga" => Url::promotes("30".to_string(), page)?.to_string(),
            "offprintmanga" => Url::filters(
                None,
                page,
                &vec![FilterValue::Select {
                    id: "類型".to_string(),
                    value: "single".to_string(),
                }],
            )?
            .to_string(),
            "finishmanga" => Url::serialization("0".to_string())?.to_string(),
            _ => bail!("Invalid listing"),
        };

        let response = Fetch::get(url)?.html()?;

        GenManga::list(&response)
    }
}

register_source!(
    Jmtt,
    BaseUrlProvider,
    PageImageProcessor,
    ImageRequestProvider,
    Home,
    ListingProvider
);

#[cfg(test)]
mod test;
