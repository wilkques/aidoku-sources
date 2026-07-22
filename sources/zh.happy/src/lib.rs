#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod crypto;
mod fetch;
mod json;
mod settings;
mod url;

use aidoku::{
    BaseUrlProvider, Chapter, FilterValue, Manga, MangaPageResult, Page, Result, Source, Viewer,
    alloc::{String, Vec},
    prelude::*,
};

use crate::json::GenManga;
use crate::url::Url;

struct Happy;

impl Source for Happy {
    fn new() -> Self {
        Self
    }

    fn get_search_manga_list(
        &self,
        query: Option<String>,
        page: i32,
        filters: Vec<FilterValue>,
    ) -> Result<MangaPageResult> {
        let base = settings::get_base_url();
        let url_obj = Url::filters(query.as_deref(), page, &filters)?;

        let data = match &url_obj {
            // Search is CF-protected. Primary: a background WebView passes the
            // automatic JS challenge then POSTs same-origin. Fallback: a foreground
            // document GET to /sssearch triggers the app's interactive CF bypass
            // dialog, user completes captcha, then POST.
            Url::Search { query } => fetch::fetch_search(&base, query)?,
            _ => fetch::fetch_list(&base, url_obj.to_string())?,
        };

        GenManga::list(data)
    }

    fn get_manga_update(
        &self,
        mut manga: Manga,
        _needs_details: bool,
        needs_chapters: bool,
    ) -> Result<Manga> {
        manga.viewer = Viewer::Webtoon;

        if needs_chapters {
            manga.chapters = Some(GenManga::chapters(&manga.key)?);
        }

        Ok(manga)
    }

    fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
        let base = settings::get_base_url();
        let json = fetch::fetch_reading(&base, &manga.key, &chapter.key)?;
        GenManga::chapter(json, &manga.key, &chapter.key)
    }
}

impl BaseUrlProvider for Happy {
    fn get_base_url(&self) -> Result<String> {
        Ok(settings::get_base_url())
    }
}

register_source!(
    Happy,
    BaseUrlProvider
);

#[cfg(test)]
mod test;
