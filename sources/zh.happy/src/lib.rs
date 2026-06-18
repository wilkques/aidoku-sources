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
    imports::{js::WebView, std::current_date},
    prelude::*,
};

use crate::fetch::Fetch;
use crate::json::{ApiResponse, GenManga, ReadingApiResponse};
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

        let data: ApiResponse = match &url_obj {
            // Search is CF-protected. Primary: a background WebView passes the
            // automatic JS challenge (same mechanism that makes reading work)
            // then POSTs same-origin. Fallback: a foreground document GET to
            // /sssearch triggers the app's interactive CF bypass dialog, then
            // POST. Use the RAW (un-encoded) query for the JSON body.
            Url::Search { .. } => {
                let raw_query = query.clone().unwrap_or_else(|| {
                    filters
                        .iter()
                        .find_map(|f| match f {
                            FilterValue::Text { value, .. } => Some(value.clone()),
                            _ => None,
                        })
                        .unwrap_or_default()
                });
                fetch_search(&base, &raw_query)?
            }
            _ => {
                Fetch::get(url_obj.to_string())?
                    .header("Referer", &format!("{}/latest", base))
                    .json_owned()?
            }
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
        let json = fetch_reading(&base, &manga.key, &chapter.key)?;
        GenManga::chapter(json, &manga.key, &chapter.key)
    }
}

// happymh returns the correct tile order only when `avifSupport`/`webpSupport`
// cookies are present. URLSession ignores a manually set Cookie header, so we
// use a background WebView to write them into the shared cookie store via JS.
// CF bypass is handled at source entry via /sssearch.
fn fetch_reading(host: &str, manga_key: &str, chapter_key: &str) -> Result<ReadingApiResponse> {
    match fetch_reading_webview(host, manga_key, chapter_key) {
        Ok(resp) => Ok(resp),
        Err(_) => fetch_reading_direct(host, manga_key, chapter_key),
    }
}

fn fetch_reading_webview(
    host: &str,
    manga_key: &str,
    chapter_key: &str,
) -> Result<ReadingApiResponse> {
    let wv = WebView::new();
    let reader = format!("{}/mangaread/{}/{}", host, manga_key, chapter_key);
    wv.load_blocking(Fetch::get(reader)?)?;
    aidoku::imports::std::sleep(1);
    let _ = wv.eval("document.cookie='avifSupport=1;path=/';document.cookie='webpSupport=1;path=/';");
    let js = format!(
        "(function(){{var x=new XMLHttpRequest();\
         x.open('GET','/v2.0/apis/manga/reading?code={}&cid={}&v=v4.300101&_t='+Date.now(),false);\
         x.setRequestHeader('x-requested-with','XMLHttpRequest');\
         x.send();return x.responseText;}})()",
        manga_key, chapter_key
    );
    let body = wv.eval(&js)?;
    serde_json::from_str::<ReadingApiResponse>(&body)
        .map_err(|_| aidoku::error!("webview reading parse failed (len={})", body.len()))
}

fn fetch_reading_direct(
    host: &str,
    manga_key: &str,
    chapter_key: &str,
) -> Result<ReadingApiResponse> {
    let ts = current_date() * 1000;
    let url = format!(
        "{}/v2.0/apis/manga/reading?code={}&cid={}&v=v4.300101&_t={}",
        host, manga_key, chapter_key, ts
    );
    Fetch::get(url)?
        .header("Referer", &format!("{}/mangaread/{}/{}", host, manga_key, chapter_key))
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Accept", "application/json")
        .header("User-Agent", &settings::get_user_agent())
        .json_owned()
}

// ── 搜尋（CF bypass）─────────────────────────────────────────
// ssearch API 受 Cloudflare 保護，需要 cf_clearance。主方案沿用 reading
// 的背景 WebView（過自動 JS 挑戰）靜默取得 clearance；失敗才退回前台 GET
// /sssearch 觸發 App 互動式驗證對話框，使用者過驗證後再 POST。
fn fetch_search(base: &str, query: &str) -> Result<ApiResponse> {
    fetch_search_webview(base, query).or_else(|_| fetch_search_interactive(base, query))
}

fn fetch_search_webview(base: &str, query: &str) -> Result<ApiResponse> {
    let wv = WebView::new();
    // 載入符合 /mangaread/* 規則的路徑觸發 CF 自動 JS 挑戰（edge 會發 cf_clearance，
    // origin 即使 404 也無妨）。不可用 /sssearch（互動式 captcha，背景過不了）。
    wv.load_blocking(Fetch::get(format!("{}/mangaread/0/0", base))?)?;
    aidoku::imports::std::sleep(2);
    let _ = wv.eval("document.cookie='avifSupport=1;path=/';document.cookie='webpSupport=1;path=/';");
    // 只 JS-escape 一次 query，body 交給 JS 端 JSON.stringify 保證合法。
    let js = format!(
        "(function(){{var q=\"{}\";var x=new XMLHttpRequest();\
         x.open('POST','/v2.0/apis/manga/ssearch',false);\
         x.setRequestHeader('Content-Type','application/json');\
         x.setRequestHeader('x-requested-with','XMLHttpRequest');\
         x.send(JSON.stringify({{searchkey:q,v:\"v2.13\"}}));\
         return x.responseText;}})()",
        escape_str(query)
    );
    let body = wv.eval(&js)?;
    if body.starts_with("<!") {
        return Err(aidoku::error!("search webview hit CF challenge"));
    }
    serde_json::from_str::<ApiResponse>(&body)
        .map_err(|_| aidoku::error!("search webview parse failed (len={})", body.len()))
}

fn fetch_search_interactive(base: &str, query: &str) -> Result<ApiResponse> {
    // document 類前台 GET，被 CF 擋時會觸發 App 的互動式 bypass dialog；
    // 使用者過驗證後 cf_clearance 進共享 cookie store。只打一次避免重複彈窗。
    let _ = Fetch::get(format!("{}/sssearch", base))?
        .header("User-Agent", &settings::get_user_agent())
        .string();
    fetch_search_direct(base, query)
}

fn fetch_search_direct(base: &str, query: &str) -> Result<ApiResponse> {
    let body = format!("{{\"searchkey\":\"{}\",\"v\":\"v2.13\"}}", escape_str(query));
    let make_request = || -> Result<String> {
        Fetch::post(format!("{}/v2.0/apis/manga/ssearch", base))?
            .header("Content-Type", "application/json")
            .header("Referer", &format!("{}/sssearch", base))
            .header("X-Requested-With", "XMLHttpRequest")
            .body(body.as_bytes())
            .string()
    };
    let mut raw = make_request()?;
    if raw.starts_with("<!") {
        raw = make_request()?;
    }
    serde_json::from_str(&raw).map_err(|_| aidoku::error!("search parse failed"))
}

/// JSON/JS 字串跳脫（同時適用 JSON body 與 eval 內的 JS 字串字面值）。
fn escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
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
