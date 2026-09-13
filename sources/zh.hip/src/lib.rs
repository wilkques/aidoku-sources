#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod decoder;
mod fetch;
mod home;
mod html;
mod settings;
mod url;

use aidoku::{
    Chapter, FilterValue, Listing, ListingProvider,
    Manga, MangaPageResult, Page, Result, Source,
    alloc::{String, Vec, string::ToString as _},
    prelude::*,
};

use crate::fetch::Fetch;
use crate::html::GenManga;
use crate::url::{FilterKind, Url};

struct Hip;

impl Source for Hip {
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

        let response = Fetch::get(url)?.html()?;

        GenManga::chapter(&response)
    }
}

impl ListingProvider for Hip {
    fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
        let kind = match listing.id.as_str() {
            "popularity" => FilterKind::Path("popularity".to_string()),
            "weekly" => FilterKind::Path("weekly".to_string()),
            "korean" => FilterKind::Category(1),
            "mainland" => FilterKind::Category(2),
            "japanese" => FilterKind::Category(3),
            "ongoing" => FilterKind::Status("ongoing".to_string()),
            _ => bail!("Invalid listing"),
        };

        let url = Url::Filter { kind, page }.to_string();

        let response = Fetch::get(url)?.html()?;

        GenManga::list(&response)
    }
}

register_source!(Hip, Home, ListingProvider);

#[cfg(test)]
mod test;
