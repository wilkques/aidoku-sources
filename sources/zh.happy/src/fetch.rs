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

// 用來觸發 CF 自動 JS 挑戰的已知真實章節(2026-07-22 起假路徑 /mangaread/0/0 開始被
// CF 判定為互動式驗證而非自動挑戰,改用真實存在的 manga/chapter,與 reading 的
// 觸發方式一致)。
const CF_TRIGGER_PATH: &str = "mangaread/weilaidegudongdian/6611564";

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
    // webview 優先(不可倒成 direct-first):圖片順序正確與否綁定在 avifSupport/webpSupport
    // cookie 上(步驟十六),這兩個 cookie 只有 webview 用 document.cookie 才能設進去,
    // direct 完全沒有辦法(手動 Cookie header 會被 URLSession 忽略,步驟 16.4)。
    // direct-first 曾經試過(cf_clearance 已有效時會直接成功解析),但拿到的是沒有
    // avif/webp cookie 的亂序資料,導致圖片順序錯誤回歸,故 revert。
    match fetch_reading_webview(host, manga_key, chapter_key) {
        Ok(resp) => Ok(resp),
        Err(e) => {
            aidoku::println!("[happy] reading webview failed: {:?} -> trying direct", e);
            fetch_reading_direct(host, manga_key, chapter_key)
        }
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
         x.open('GET','/v2.0/apis/manga/reading?code={}&cid={}&v=v4.300102&_t='+Date.now(),false);\
         x.setRequestHeader('x-requested-with','XMLHttpRequest');\
         x.send();return x.responseText;}})()",
        manga_key, chapter_key
    );
    let body = wv.eval(&js)?;
    serde_json::from_str::<ReadingApiResponse>(&body).map_err(|_| {
        aidoku::error!(
            "webview reading parse failed (len={}): {}",
            body.len(),
            truncate_str(&body, 200)
        )
    })
}

fn fetch_reading_direct(
    host: &str,
    manga_key: &str,
    chapter_key: &str,
) -> Result<ReadingApiResponse> {
    let ts = current_date() * 1000;
    let url = format!(
        "{}/v2.0/apis/manga/reading?code={}&cid={}&v=v4.300102&_t={}",
        host, manga_key, chapter_key, ts
    );
    let raw = Fetch::get(url)?
        .header("Referer", &format!("{}/mangaread/{}/{}", host, manga_key, chapter_key))
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Accept", "application/json")
        .header("User-Agent", &settings::get_user_agent())
        .string()?;
    serde_json::from_str(&raw).map_err(|_| {
        aidoku::error!(
            "reading direct parse failed (len={}): {}",
            raw.len(),
            truncate_str(&raw, 200)
        )
    })
}

// ── 漫畫列表 / 篩選（CF bypass）───────────────────────────────
// /apis/c/index 原本被視為非 CF 保護端點，直接 Fetch::get + json_owned()。
// 2026-07-21 回報 get_search_manga_list 手機上直接顯示重試(JsonParseError)，
// 研判該端點現在也會被 CF 擋(回人機驗證 HTML，json_owned 直接解析失敗)。
// 沿用 search 已驗證的策略:直接請求優先(便宜、多數情況仍可行)，失敗或收到
// CF 頁再退背景 WebView(過自動 JS 挑戰)，最後才前台互動式 fallback。

pub fn fetch_list(base: &str, url: String) -> Result<ApiResponse> {
    match fetch_list_direct(&url, base) {
        Ok(resp) => Ok(resp),
        Err(e) => {
            aidoku::println!("[happy] list direct failed: {:?} -> trying webview", e);
            fetch_list_webview(base, &url).or_else(|e2| {
                aidoku::println!("[happy] list webview failed: {:?} -> trying interactive", e2);
                fetch_list_interactive(base, &url)
            })
        }
    }
}

fn fetch_list_direct(url: &str, base: &str) -> Result<ApiResponse> {
    let raw = Fetch::get(String::from(url))?
        .header("Referer", &format!("{}/latest", base))
        .string()?;
    if raw.starts_with("<!") {
        return Err(aidoku::error!("list direct hit CF challenge (len={})", raw.len()));
    }
    serde_json::from_str(&raw)
        .map_err(|_| aidoku::error!("list direct parse failed (len={})", raw.len()))
}

fn fetch_list_webview(base: &str, url: &str) -> Result<ApiResponse> {
    let wv = WebView::new();
    // 同 search:載入真實存在的章節路徑觸發 CF 自動 JS 挑戰取得 cf_clearance。
    wv.load_blocking(Fetch::get(format!("{}/{}", base, CF_TRIGGER_PATH))?)?;
    sleep(2);
    inject_image_cookies(&wv);
    let js = format!(
        "(function(){{var x=new XMLHttpRequest();\
         x.open('GET','{}',false);\
         x.setRequestHeader('x-requested-with','XMLHttpRequest');\
         x.send();return x.responseText;}})()",
        url
    );
    let body = wv.eval(&js)?;
    if body.starts_with("<!") {
        return Err(aidoku::error!("list webview hit CF challenge (len={})", body.len()));
    }
    serde_json::from_str::<ApiResponse>(&body)
        .map_err(|_| aidoku::error!("list webview parse failed (len={})", body.len()))
}

fn fetch_list_interactive(base: &str, url: &str) -> Result<ApiResponse> {
    // 前台 document 類 GET 觸發 App 互動式 bypass dialog(沿用 search 已驗證會跳出的 /sssearch)，
    // 使用者過驗證後再重打實際 list 端點。
    let _ = Fetch::get(format!("{}/sssearch", base))?
        .header("User-Agent", &settings::get_user_agent())
        .string();
    fetch_list_direct(url, base)
}

// ── 搜尋（CF bypass）─────────────────────────────────────────
// ssearch API 受 Cloudflare 保護，需要 cf_clearance；同時伺服器另外檢查
// Referer 必須是 /sssearch，不符合就回 200 + {"status":400,"msg":"not support"}
// （2026-07-23 發現，見 RESEARCH.md 步驟）。背景 WebView 用 XHR 發送時，
// Referer 由瀏覽器自動帶成當時載入的頁面網址，JS 無法覆寫（forbidden header），
// 結構上不可能滿足這個檢查，故不再嘗試 webview 路線。
// direct 優先（原生 Request 可自訂 Referer，cf_clearance 若已在共享 cookie
// store 內有效就會直接成功，不需要跳驗證視窗）；失敗才退回前台 GET /sssearch
// 觸發 App 互動式驗證對話框，使用者過驗證後再重試 POST。

pub fn fetch_search(base: &str, query: &str) -> Result<ApiResponse> {
    match fetch_search_direct(base, query) {
        Ok(resp) => Ok(resp),
        Err(e) => {
            aidoku::println!("[happy] search direct failed: {:?} -> trying interactive", e);
            fetch_search_interactive(base, query)
        }
    }
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
    let body = build_search_body(query);
    let make_request = || -> Result<String> {
        let ts = current_date() * 1000;
        Fetch::post(format!("{}/v2.0/apis/manga/ssearch", base))?
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .header("Referer", &format!("{}/sssearch", base))
            .header("X-Requested-With", "XMLHttpRequest")
            .header("X-Requested-Id", &format!("{}", ts))
            .body(body.as_bytes())
            .string()
    };
    let mut raw = make_request()?;
    if raw.starts_with("<!") {
        raw = make_request()?;
    }
    serde_json::from_str(&raw).map_err(|_| {
        aidoku::error!(
            "search parse failed (len={}): {}",
            raw.len(),
            truncate_str(&raw, 200)
        )
    })
}

// ── Helpers ──────────────────────────────────────────────────

/// 截斷字串到最多 max bytes，並回退到最近的合法 UTF-8 char boundary(避免中文等
/// 多 byte 字元被從中間切斷造成 panic)。
fn truncate_str(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

fn inject_image_cookies(wv: &WebView) {
    let _ = wv.eval(
        "document.cookie='avifSupport=1;path=/';document.cookie='webpSupport=1;path=/';",
    );
}

/// 搜尋請求的 form-urlencoded body。真實網站送的欄位是
/// `searchkey`、`v`、`s`、`d`（`s=web`、`d=` 空值），不是 JSON。
fn build_search_body(query: &str) -> String {
    format!("searchkey={}&v=v2.13&s=web&d=", form_url_encode(query))
}

/// `application/x-www-form-urlencoded` percent-encoding(等同 JS `encodeURIComponent`):
/// 逐 UTF-8 byte 編碼,非英數字與 `-_.~` 一律轉成 `%XX`。
fn form_url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
