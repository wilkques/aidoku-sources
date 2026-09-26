#![cfg_attr(not(test), no_std)]
extern crate alloc;

mod fetch;
mod home;
mod html;
mod settings;
mod url;

use aidoku::{
    BaseUrlProvider, Chapter, DynamicSettings, FilterValue, ImageResponse, Listing,
    ListingProvider, Manga, MangaPageResult, Page, PageContext, PageImageProcessor, Result, Setting,
    Source,
    alloc::{String, Vec, string::ToString as _, vec},
    imports::canvas::ImageRef,
    prelude::*,
};

use crate::fetch::{Fetch, Web};
use crate::html::GenManga;
use crate::url::Url;

struct Bakamh;

impl Source for Bakamh {
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

        Fetch::list(url)
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
        let url = Url::chapter(chapter.key.clone())?.to_string();

        let response = Fetch::html(url)?;

        GenManga::chapter(&response)
    }
}

impl BaseUrlProvider for Bakamh {
    fn get_base_url(&self) -> Result<String> {
        Ok(settings::get_base_url())
    }
}

impl ListingProvider for Bakamh {
    fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
        let filters = match listing.id.as_str() {
            "dailymanga" => vec![FilterValue::Sort {
                id: "排序".to_string(),
                index: 1,
                ascending: false,
            }],
            "rankmanga" => vec![FilterValue::Sort {
                id: "排序".to_string(),
                index: 2,
                ascending: false,
            }],
            "viewmanga" => vec![FilterValue::Sort {
                id: "排序".to_string(),
                index: 3,
                ascending: false,
            }],
            "newmanga" => vec![FilterValue::Sort {
                id: "排序".to_string(),
                index: 4,
                ascending: false,
            }],
            _ => bail!("Invalid listing"),
        };

        let url = Url::filters(None, page, &filters)?.to_string();

        Fetch::list(url)
    }
}

// 章節圖片跟網站在同一個 CF zone，app 用 URLSession 下載一定被擋。載入失敗時閱讀器會帶著
// 失敗的請求呼叫這裡（Aidoku `ReaderPageView` / `ReaderWebtoonPageNode` 的 processWithoutImage），
// 改用帶著 clearance 的 WebView 抓回來。成功載入的圖片（例如已快取）直接原樣回傳。
impl PageImageProcessor for Bakamh {
    fn process_page_image(
        &self,
        response: ImageResponse,
        _context: Option<PageContext>,
    ) -> Result<ImageRef> {
        if (200..300).contains(&response.code) {
            return Ok(response.image);
        }

        let url = response
            .request
            .url
            .ok_or_else(|| error!("No image url"))?;

        // 閱讀器用 `try?` 呼叫這裡，錯誤不會顯示在畫面上，只能靠 log 查
        let data = Web::new()
            .and_then(|web| web.image(&url))
            .inspect_err(|error| aidoku::println!("[baka] page image failed: {:?}", error))?;

        Ok(ImageRef::new(&data))
    }
}

impl DynamicSettings for Bakamh {
    fn get_dynamic_settings(&self) -> Result<Vec<Setting>> {
        Ok(settings::get_cf_settings())
    }
}

register_source!(
    Bakamh,
    BaseUrlProvider,
    ListingProvider,
    Home,
    DynamicSettings,
    PageImageProcessor
);
