use aidoku::{
    Manga, MangaPageResult, Result, Viewer,
    alloc::{String, Vec, string::ToString as _},
    imports::net::Response,
    prelude::*,
};
use serde::Deserialize;

use crate::{fetch::Fetch, settings, url::Url};

// 章節列表不在 detail 頁的靜態 HTML 裡，是前端另外打 API 拿的
// GET {api_url}/v1/manga/chapters?mid={mid}&page={page}&per_page={per_page}&order=desc
// order=desc（新到舊）跟網站預設排序一致，最新一話排最前面
#[derive(Deserialize)]
pub struct ChaptersApiResponse {
    pub data: ChaptersApiData,
}

#[derive(Deserialize)]
pub struct ChaptersApiData {
    pub total_pages: i32,
    pub items: Vec<ChapterApiItem>,
}

#[derive(Deserialize)]
pub struct ChapterApiItem {
    pub hid: String,
    pub chapter_number: f32,
    pub title: String,
    pub updated_at: String,
}

// genre/category/status 篩選、搜尋都不是走 m.hipmh.com 的 HTML 頁，是打
// hipapi1.s3file.top 回 JSON（跟 popularity/weekly 那種還是 HTML 頁完全不同），
// 要另外解析。兩個端點的 JSON 形狀不一樣（欄位是 mid 還是 id、項目在
// data.items 還是 data.data），但轉成 Manga 的邏輯完全相同，用 trait 共用。
trait ApiMangaItem {
    fn key(&self) -> &str;
    fn title(self) -> String;
    fn vertical_image_url(&self) -> &str;

    fn into_manga(self) -> Manga
    where
        Self: Sized,
    {
        let cover = Some(format!(
            "{}{}",
            settings::get_cover_url(),
            self.vertical_image_url()
        ));
        let url = Url::book(self.key().to_string()).ok().map(|u| u.to_string());
        let key = self.key().to_string();

        Manga {
            cover,
            url,
            title: self.title(),
            key,
            viewer: Viewer::Webtoon,
            ..Default::default()
        }
    }
}

fn build_manga_page_result<T: ApiMangaItem>(
    page: i32,
    total_pages: i32,
    items: Vec<T>,
) -> MangaPageResult {
    MangaPageResult {
        entries: items.into_iter().map(ApiMangaItem::into_manga).collect(),
        has_next_page: page < total_pages,
    }
}

// GET {api_url}/v1/mangas?genre|category|status=...&sort=updated&page={page}&page_size={page_size}
#[derive(Deserialize)]
struct MangaListApiResponse {
    data: MangaListApiData,
}

#[derive(Deserialize)]
struct MangaListApiData {
    page: i32,
    total_pages: i32,
    items: Vec<MangaListApiItem>,
}

#[derive(Deserialize)]
struct MangaListApiItem {
    mid: String,
    title: String,
    vertical_image_url: String,
}

impl ApiMangaItem for MangaListApiItem {
    fn key(&self) -> &str {
        &self.mid
    }

    fn title(self) -> String {
        self.title
    }

    fn vertical_image_url(&self) -> &str {
        &self.vertical_image_url
    }
}

fn manga_list_from_api_data(data: MangaListApiData) -> MangaPageResult {
    build_manga_page_result(data.page, data.total_pages, data.items)
}

/// 用在已經 `.send()` 出去、還沒決定怎麼解析的 Response（例如 home.rs 平行發送的情況）
pub(crate) fn parse_manga_list_json(response: Response) -> Result<MangaPageResult> {
    let data: MangaListApiResponse = response.get_json_owned()?;

    Ok(manga_list_from_api_data(data.data))
}

/// 用在單一 URL 直接 fetch + 解析的情況（ListingProvider）
pub(crate) fn fetch_manga_list_json(url: String) -> Result<MangaPageResult> {
    let data: MangaListApiResponse = Fetch::get(url)?.json_owned()?;

    Ok(manga_list_from_api_data(data.data))
}

/// 純字串版本，離線測試用，不打網路
pub(crate) fn parse_manga_list_json_str(json: &str) -> Option<MangaPageResult> {
    let data: MangaListApiResponse = serde_json::from_str(json).ok()?;

    Some(manga_list_from_api_data(data.data))
}

// 搜尋是另一個完全不同形狀的 JSON（欄位是 id 不是 mid，項目在 data.data 不是 data.items）
// GET {api_url}/v1/search?q={query}&page={page}&page_size={page_size}
#[derive(Deserialize)]
struct SearchApiResponse {
    data: SearchApiData,
}

#[derive(Deserialize)]
struct SearchApiData {
    page: i32,
    total_pages: i32,
    data: Vec<SearchApiItem>,
}

#[derive(Deserialize)]
struct SearchApiItem {
    id: String,
    title: String,
    vertical_image_url: String,
}

impl ApiMangaItem for SearchApiItem {
    fn key(&self) -> &str {
        &self.id
    }

    fn title(self) -> String {
        self.title
    }

    fn vertical_image_url(&self) -> &str {
        &self.vertical_image_url
    }
}

fn search_list_from_api_data(data: SearchApiData) -> MangaPageResult {
    build_manga_page_result(data.page, data.total_pages, data.data)
}

pub(crate) fn fetch_search_json(url: String) -> Result<MangaPageResult> {
    let data: SearchApiResponse = Fetch::get(url)?.json_owned()?;

    Ok(search_list_from_api_data(data.data))
}

/// 純字串版本，離線測試用，不打網路
pub(crate) fn parse_search_json_str(json: &str) -> Option<MangaPageResult> {
    let data: SearchApiResponse = serde_json::from_str(json).ok()?;

    Some(search_list_from_api_data(data.data))
}
