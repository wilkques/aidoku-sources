#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod fetch;
mod home;
mod html;
mod js_packer;
mod settings;
mod url;

use aidoku::{
    BaseUrlProvider, Chapter, FilterValue, ImageRequestProvider, Listing, ListingProvider, Manga,
    MangaPageResult, Page, PageContext, Result, Source,
    alloc::{String, Vec, string::ToString as _},
    imports::{html::Document, net::Request},
    prelude::*,
};

use crate::fetch::Fetch;
use crate::html::GenManga;
use crate::url::Url;

struct Dm5;

impl Source for Dm5 {
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
            manga.chapters = Some(GenManga::chapters(&response)?);
        }

        Ok(manga)
    }

    fn get_page_list(&self, _: Manga, chapter: Chapter) -> Result<Vec<Page>> {
        let url = Url::chapter(chapter.key.clone())?.to_string();

        let response = Fetch::get(url.clone())?.string()?;

        <Document as GenManga>::chapter(url, response)
    }
}

impl BaseUrlProvider for Dm5 {
    fn get_base_url(&self) -> Result<String> {
        Ok(settings::get_base_url())
    }
}

impl ImageRequestProvider for Dm5 {
    fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
        let cid = url
            .split("cid=")
            .nth(1)
            .and_then(|s| s.split('&').next())
            .unwrap_or("");

        let referer = if cid.is_empty() {
            url.clone()
        } else {
            Url::chapter(format!("m{}", cid))?.to_string()
        };

        Ok(Fetch::get(url)?
            .header("Accept-Language", "zh-TW")
            .header("Referer", &referer))
    }
}

impl ListingProvider for Dm5 {
    fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
        let rank = match listing.id.as_str() {
            "dailymanga" => "new",
            "cnmanga" => "1",
            "jpmanga" => "2",
            "allmanga" => "3",
            "risemanga" => "7",
            _ => bail!("Invalid listing"),
        };

        let url = Url::rank(rank.to_string())?.to_string();

        let response = Fetch::get(url)?.html()?;

        GenManga::list(&response)
    }
}

register_source!(
    Dm5,
    BaseUrlProvider,
    ImageRequestProvider,
    Home,
    ListingProvider
);

#[cfg(test)]
mod test;
