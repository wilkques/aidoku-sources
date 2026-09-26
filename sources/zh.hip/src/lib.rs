#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod decoder;
mod fetch;
mod home;
mod html;
mod json;
mod settings;
mod url;

use aidoku::{
    AidokuError, Chapter, FilterValue, Listing, ListingProvider,
    Manga, MangaPageResult, Page, PageContent, Result, Source,
    alloc::{String, Vec, string::ToString as _, vec},
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
        let built = Url::filters(query.as_deref(), page, &filters)?;

        if let Url::Search { .. } = &built {
            return json::fetch_search_json(built.to_string());
        }

        if let Url::Filter { kind, .. } = &built {
            if kind.is_json_api() {
                return json::fetch_manga_list_json(built.to_string());
            }
        }

        let response = Fetch::html(built.to_string())?;

        GenManga::list(&response)
    }

    fn get_manga_update(
        &self,
        mut manga: Manga,
        needs_details: bool,
        needs_chapters: bool,
    ) -> Result<Manga> {
        let url = Url::book(manga.key.clone())?.to_string();

        let response = Fetch::html(url)?;

        if needs_details {
            GenManga::detail(&response, &mut manga)?;
        }

        if needs_chapters {
            manga.chapters = Some(GenManga::chapters(&response)?);
        }

        Ok(manga)
    }

    fn get_page_list(&self, _: Manga, chapter: Chapter) -> Result<Vec<Page>> {
        // Aidoku 閱讀器用 `try?` 呼叫這裡（`ReaderPagedViewModel.getPages`），錯誤訊息整個被吞掉：
        // 捲到最後一頁接下一章時只會卡住，什麼提示都沒有（例如 CF 驗證失效）。
        // 所以失敗時改回傳一頁文字頁，把原因直接顯示在閱讀器裡。
        match fetch_page_list(&chapter) {
            Ok(pages) => Ok(pages),
            Err(error) => Ok(vec![error_page(error)]),
        }
    }
}

fn fetch_page_list(chapter: &Chapter) -> Result<Vec<Page>> {
    let url = Url::chapter(chapter.key.clone())?.to_string();

    let response = Fetch::html(url)?;

    GenManga::chapter(&response)
}

fn error_page(error: AidokuError) -> Page {
    let message = match error {
        AidokuError::Message(message) => message,
        other => format!("{:?}", other),
    };

    Page {
        content: PageContent::Text(format!(
            "**本章載入失敗**\n\n{}\n\n處理完後，請關閉閱讀器再重新開啟這一章。",
            message
        )),
        ..Default::default()
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

        let use_json = kind.is_json_api();
        let url = Url::Filter { kind, page }.to_string();

        if use_json {
            return json::fetch_manga_list_json(url);
        }

        let response = Fetch::html(url)?;

        GenManga::list(&response)
    }
}

register_source!(Hip, Home, ListingProvider);

#[cfg(test)]
mod test;
