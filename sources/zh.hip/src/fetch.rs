use aidoku::{
    Result,
    alloc::{String, string::ToString as _},
    imports::{
        html::{Document, Html},
        js::WebView,
        net::{HttpMethod, Request, Response},
        std::sleep,
    },
    prelude::*,
};
use serde::{Deserialize, de::DeserializeOwned};

use crate::settings;

pub struct Fetch;

impl Fetch {
    pub fn request(url: String, method: HttpMethod) -> Result<Request> {
        let user_agent = settings::get_user_agent();

        Ok(Request::new(url, method)?.header("User-Agent", &user_agent))
    }

    pub fn get(url: String) -> Result<Request> {
        Fetch::request(url, HttpMethod::Get)
    }

    pub fn head(url: String) -> Result<Request> {
        Fetch::request(url, HttpMethod::Head)
    }

    /// 送出請求，並在交給解析器之前先攔下 Cloudflare 的驗證頁
    pub fn send(request: Request) -> Result<Response> {
        let response = request.send()?;

        check_cf_challenge(&response)?;

        Ok(response)
    }

    pub fn html(url: String) -> Result<Document> {
        let response = Fetch::send(Fetch::get(url)?)?;

        // 帶上最終網址，頁面裡的相對連結才解得開（跟過重導向後可能換了網域）
        let base_url = response.get_url();
        let data = response.get_data()?;

        let document = match base_url {
            Some(base_url) => Html::parse_with_url(data, base_url)?,
            None => Html::parse(data)?,
        };

        Ok(document)
    }
}

// hipapi1.s3file.top 的 JSON API 一律從 WebView 裡發請求，不走 `Request::send()`
// （完整推導見 docs/RESEARCH.md 第十節）：
//
// - CF 對這個 zone 只信任「解過驗證的那種客戶端」。實機上使用者在 WKWebView 解完驗證、
//   app 再用 URLSession 帶著同一張 cf_clearance 重打照樣被擋（8.10、9.4.2）——所以要讓
//   請求也從 WKWebView 發出去。
// - 設定頁的「Cloudflare 驗證」按鈕（settings.json 的 login/web）開的 WebView，跟這裡的
//   `WebView::new()` 用的是同一個 per-source cookie store（Aidoku 的
//   `Settings/WebView.swift` 與 AidokuRunner 的 `WebViewHandler.swift` 都是
//   `.forSource(key:)`），也都沒有自訂 UA。使用者在那裡手動解出來的 cf_clearance，這裡
//   的 XHR 會自動帶上。
// - 用 `load_html` 把空白文件的網址設成 API 網域本身，XHR 就是同源請求：cookie 算第一方、
//   response header 也讀得到，而且這一步不會發出任何網路請求。
// - 順帶避開 host app 的 CloudflareHandler：`Request::send()` 被擋時 app 會跳驗證彈窗，
//   但解完後的重試走 URLSession，一定失敗（9.4.2），只是白白打擾使用者。
pub struct Api {
    webview: WebView,
}

const MAX_ATTEMPTS: u32 = 3;

// eval 回傳值只能是字串，所以在 JS 端把狀態碼、header 跟內容包成一個 JSON 帶回來
#[derive(Deserialize)]
struct XhrResult {
    status: i32,
    mitigated: Option<String>,
    body: String,
}

impl Api {
    pub fn new() -> Result<Self> {
        let webview = WebView::new();
        let base_url = format!("{}/", settings::get_api_url());

        webview.load_html_blocking("<!DOCTYPE html><html><body></body></html>", Some(&base_url))?;

        Ok(Self { webview })
    }

    pub fn json<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        // 網址用 JSON 字串字面值嵌進 JS，搜尋字串裡有引號之類的字元也不會壞掉
        let url_literal = serde_json::to_string(url).map_err(|_| error!("Invalid url"))?;

        let js = format!(
            "(function(){{try{{var x=new XMLHttpRequest();x.open('GET',{},false);x.send();\
             return JSON.stringify({{status:x.status,mitigated:x.getResponseHeader('cf-mitigated'),body:x.responseText}});\
             }}catch(e){{return JSON.stringify({{status:0,mitigated:null,body:String(e)}});}}}})()",
            url_literal
        );

        // 偶發被擋時先靜默重試，都失敗才叫使用者去設定頁驗證。只有 CF 挑戰才重試，其他錯誤
        // 重打也不會變好。上限固定、間隔 1 秒：WebView 內無上限的輪詢曾讓 host app 崩潰
        // （zh.baka 的教訓）。沿用同一個 WebView 就好，重建也是同一個 cookie store。
        let mut attempt = 1;

        let result = loop {
            let result = self.request(&js)?;

            if !is_cf_challenge(result.status, result.mitigated.as_deref()) {
                break result;
            }

            if attempt >= MAX_ATTEMPTS {
                bail!(
                    "站方 Cloudflare 驗證已失效（已自動重試 {} 次）。請到 瀏覽 → 嬉皮 → 齒輪 →「Cloudflare 驗證」完成驗證後再試",
                    MAX_ATTEMPTS
                );
            }

            attempt += 1;
            sleep(1);
        };

        if result.status != 200 {
            bail!("API 請求失敗（HTTP {}）：{}", result.status, truncate(&result.body, 200));
        }

        serde_json::from_str(&result.body)
            .map_err(|_| error!("API 回應解析失敗：{}", truncate(&result.body, 200)))
    }

    fn request(&self, js: &str) -> Result<XhrResult> {
        let raw = self.webview.eval(js)?;

        serde_json::from_str(&raw).map_err(|_| error!("WebView 請求失敗：{}", truncate(&raw, 200)))
    }
}

fn truncate(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((index, _)) => format!("{}…", &text[..index]),
        None => text.to_string(),
    }
}

// Cloudflare 擋下來時回的是 403 + 一張「Just a moment...」的 HTML 驗證頁。直接把它餵給
// JSON/HTML 解析器，使用者只會看到莫名其妙的 parse error，完全不知道發生什麼事，所以在
// 解析前先攔下來換成看得懂的訊息。
//
// 判斷依據用 `cf-mitigated` header 而不是比對 body 裡的「Just a moment」字串：前者是
// Cloudflare 平台本身的標準 header，不像 body 措辭會隨挑戰版本改變（作法取自社群來源
// en.comix，見 docs/RESEARCH.md 8.7）。
//
// 這裡只給 m.hipmh.com／reader.hipmh.top 的 HTML 頁用（實測都是長 TTL 的 CDN 快取，很少被擋）；
// hipapi1 的 JSON API 走上面的 `Api`。
pub fn check_cf_challenge(response: &Response) -> Result<()> {
    let mitigated = response.get_header("cf-mitigated");

    if is_cf_challenge(response.status_code(), mitigated.as_deref()) {
        bail!("站方 Cloudflare 驗證中，請稍候幾分鐘再試");
    }

    Ok(())
}

// 判斷邏輯拆成純函式，離線測試才驗證得到：`Response` 綁著 host 給的 rid handle，
// 測試環境裡造不出來。
pub(crate) fn is_cf_challenge(status_code: i32, cf_mitigated: Option<&str>) -> bool {
    // 403 與 503 兩種都算，跟 host app 的判斷一致
    // （Aidoku 的 CloudflareHandler.swift：`blockedStatusCodes: Set<Int> = [403, 503]`）
    matches!(status_code, 403 | 503)
        && cf_mitigated.is_some_and(|value| value.eq_ignore_ascii_case("challenge"))
}
