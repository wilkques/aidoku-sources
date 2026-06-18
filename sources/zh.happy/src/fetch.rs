use aidoku::{
    Result,
    alloc::String,
    imports::{
        js::WebView,
        net::{HttpMethod, Request},
        std::{current_date, sleep},
    },
    prelude::format,
};

use crate::{json::{ApiResponse, ReadingApiResponse}, settings};

pub struct Fetch;

impl Fetch {
    pub fn request(url: String, method: HttpMethod) -> Result<Request> {
        Ok(Request::new(url, method)?)
    }

    pub fn get(url: String) -> Result<Request> {
        Fetch::request(url, HttpMethod::Get)
    }

    pub fn post(url: String) -> Result<Request> {
        Fetch::request(url, HttpMethod::Post)
    }
}

// ── 閱讀 API ─────────────────────────────────────────────────

pub fn fetch_reading(host: &str, manga_key: &str, chapter_key: &str) -> Result<ReadingApiResponse> {
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
    sleep(1);
    inject_image_cookies(&wv);
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

pub fn fetch_search(base: &str, query: &str) -> Result<ApiResponse> {
    fetch_search_webview(base, query).or_else(|_| fetch_search_interactive(base, query))
}

fn fetch_search_webview(base: &str, query: &str) -> Result<ApiResponse> {
    let wv = WebView::new();
    // 載入符合 /mangaread/* 規則的路徑觸發 CF 自動 JS 挑戰（edge 會發 cf_clearance，
    // origin 即使 404 也無妨）。不可用 /sssearch（互動式 captcha，背景過不了）。
    wv.load_blocking(Fetch::get(format!("{}/mangaread/0/0", base))?)?;
    sleep(2);
    inject_image_cookies(&wv);
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

// ── Helpers ──────────────────────────────────────────────────

fn inject_image_cookies(wv: &WebView) {
    let _ = wv.eval(
        "document.cookie='avifSupport=1;path=/';document.cookie='webpSupport=1;path=/';",
    );
}

/// JSON/JS 字串跳脫（同時適用 JSON body 與 eval 內的 JS 字串字面值）。
pub fn escape_str(s: &str) -> String {
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
