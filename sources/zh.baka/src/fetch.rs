use aidoku::{
    Manga, MangaPageResult, Result,
    alloc::{String, Vec, string::ToString as _},
    imports::{
        html::{Document, Html},
        js::WebView,
        std::sleep,
    },
    prelude::*,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use serde::Deserialize;

use crate::{html::GenManga, settings};

pub struct Fetch;

impl Fetch {
    /// 單次抓一個頁面；要連續抓好幾頁的話（例如首頁）改用同一個 `Web`
    pub fn html(url: String) -> Result<Document> {
        Web::new()?.html(&url)
    }

    /// 單次抓一頁列表（含封面），見 `Web::list`
    pub fn list(url: String) -> Result<MangaPageResult> {
        Web::new()?.list(&url)
    }
}

// 全站（bakamh.com/.ru、baka3.me）都在 Cloudflare 的 interactive 挑戰後面，`Request::send()`
// 一律被擋。作法跟 zh.hip 已實機驗證成功的解法相同（見 zh.hip/docs/RESEARCH.md 第十節）：
//
// - 設定頁的「Cloudflare 驗證」按鈕（settings.json 的 login/web）開的 WebView，跟這裡的
//   `WebView::new()` 共用同一個 per-source cookie store，也都沒有自訂 UA。使用者在那個
//   看得見的視窗裡手動點過驗證，cf_clearance 就會存進來源自己的 store。
// - 頁面改從 WebView 裡用同源 XHR 抓：`load_html` 把空白文件的網址設成目前的網域
//   （這一步不發網路請求），XHR 就會帶上那張 clearance。CF 信任的是解驗證的那種客戶端
//   （WKWebView），URLSession 帶同一張 cookie 還是會被擋（hip 9.4.2）。
// - 不要在 XHR 上自訂 User-Agent：clearance 綁 UA，必須跟登入視窗的 WKWebView 預設 UA 一致。
//   （舊的「驗證後 WebView 仍被擋」實驗，很可能就是因為導覽請求帶了自訂的桌機 Chrome UA。）
pub struct Web {
    webview: WebView,
}

// 偶發被擋時先靜默重試，都失敗才叫使用者去設定頁。上限固定、間隔 1 秒：WebView 內
// 無上限的輪詢曾讓 host app 崩潰重啟。
const MAX_ATTEMPTS: u32 = 3;

// `images()` 每張圖的結果：成功是 {t: content-type, d: base64}，失敗是 {e: 原因}
#[derive(Deserialize)]
struct ImageResult {
    t: Option<String>,
    d: Option<String>,
    e: Option<String>,
}

impl ImageResult {
    fn into_result(self) -> core::result::Result<(String, String), String> {
        match (self.t, self.d) {
            (Some(content_type), Some(data)) => Ok((content_type, data)),
            _ => Err(self.e.unwrap_or_else(|| "unknown".to_string())),
        }
    }
}

// eval 回傳值只能是字串，所以在 JS 端把狀態碼、header 跟內容包成一個 JSON 帶回來
#[derive(Deserialize)]
struct XhrResult {
    status: i32,
    mitigated: Option<String>,
    body: String,
}

impl Web {
    pub fn new() -> Result<Self> {
        let webview = WebView::new();
        let base_url = format!("{}/", settings::get_base_url());

        webview.load_html_blocking("<!DOCTYPE html><html><body></body></html>", Some(&base_url))?;

        Ok(Self { webview })
    }

    pub fn html(&self, url: &str) -> Result<Document> {
        // 網址用 JSON 字串字面值嵌進 JS，搜尋字串裡有引號之類的字元也不會壞掉
        let url_literal = serde_json::to_string(url).map_err(|_| error!("Invalid url"))?;

        let js = format!(
            "(function(){{try{{var x=new XMLHttpRequest();x.open('GET',{},false);x.send();\
             return JSON.stringify({{status:x.status,mitigated:x.getResponseHeader('cf-mitigated'),body:x.responseText}});\
             }}catch(e){{return JSON.stringify({{status:0,mitigated:null,body:String(e)}});}}}})()",
            url_literal
        );

        let mut attempt = 1;

        let result = loop {
            let result = self.request(&js)?;

            if !is_cf_challenge(result.status, result.mitigated.as_deref()) {
                break result;
            }

            if attempt >= MAX_ATTEMPTS {
                bail!(
                    "站方 Cloudflare 驗證已失效（已自動重試 {} 次）。請到 瀏覽 → 巴卡漫畫 → 齒輪 →「Cloudflare 驗證（{}）」完成驗證後再試",
                    MAX_ATTEMPTS,
                    settings::cf_host(&settings::get_base_url())
                );
            }

            attempt += 1;
            sleep(1);
        };

        if result.status != 200 {
            bail!("頁面請求失敗（HTTP {}）：{}", result.status, truncate(&result.body, 200));
        }

        Ok(Html::parse_with_url(result.body, url)?)
    }

    /// 抓一頁列表，並把封面換成 WebView 抓回來的 `data:` 網址（原因見 `inline_covers`）
    pub fn list(&self, url: &str) -> Result<MangaPageResult> {
        let mut result = self.html(url)?.list()?;

        self.inline_covers(&mut result.entries);

        Ok(result)
    }

    // 封面跟網站在同一個 CF zone（`/wp-content/uploads/`），app 用 URLSession 下載一定被擋，
    // 而且封面載入失敗時 app 不會回頭呼叫來源（Aidoku `MangaGridCell` 只走 `CoverRecovery`）。
    // 所以趁手上有帶著 clearance 的 WebView，把整頁封面平行抓完，轉成 `data:` 網址直接交給
    // app。封面是 193x278 左右的縮圖，一頁的量可以接受。抓不到的保留原網址，不讓列表壞掉。
    pub fn inline_covers(&self, mangas: &mut [Manga]) {
        let targets: Vec<(usize, String)> = mangas
            .iter()
            .enumerate()
            .filter_map(|(index, manga)| {
                manga
                    .cover
                    .as_ref()
                    .filter(|cover| cover.starts_with("http"))
                    .map(|cover| (index, cover.clone()))
            })
            .collect();

        if targets.is_empty() {
            return;
        }

        let urls: Vec<String> = targets.iter().map(|(_, url)| url.clone()).collect();

        let images = match self.images(&urls) {
            Ok(images) => images,
            Err(error) => {
                aidoku::println!("[baka] cover fetch failed: {:?}", error);
                return;
            }
        };

        let total = images.len();
        let mut inlined = 0;
        let mut first_error = None;

        for ((index, _), image) in targets.into_iter().zip(images) {
            match image.into_result() {
                Ok((content_type, data)) => {
                    mangas[index].cover = Some(format!("data:{};base64,{}", content_type, data));
                    inlined += 1;
                }
                Err(error) => first_error = first_error.or(Some(error)),
            }
        }

        // 封面失敗是靜默的（保留原網址、不讓列表壞掉），所以把結果留在 log 才查得到原因
        if inlined < total {
            aidoku::println!(
                "[baka] covers inlined {}/{}, first error: {}",
                inlined,
                total,
                first_error.unwrap_or_default()
            );
        }
    }

    /// 抓單張圖的原始位元組（給章節圖片的 `PageImageProcessor` 用）
    pub fn image(&self, url: &str) -> Result<Vec<u8>> {
        let image = self
            .images(&[url.to_string()])?
            .into_iter()
            .next()
            .ok_or_else(|| error!("No image result"))?;

        let (_, data) = image.into_result().map_err(|reason| {
            error!(
                "圖片下載失敗（{}）。若持續發生，請到 瀏覽 → 巴卡漫畫 → 齒輪 →「Cloudflare 驗證（{}）」重新驗證",
                reason,
                settings::cf_host(&settings::get_base_url())
            )
        })?;

        BASE64_STANDARD
            .decode(data)
            .map_err(|_| error!("圖片資料解碼失敗"))
    }

    // 依序抓多張圖（同一次 eval 內跑完）。eval 只能回傳字串，所以在 JS 端把位元組轉成 base64
    // 再帶回來；失敗的那張帶回原因（HTTP 狀態碼／例外訊息）。
    //
    // 用同步 XHR 而不是 `eval_async` + `fetch`：這版 Aidoku 的 WebView `eval_async` 拿不到結果
    // （`WebViewHandler.evaluateAsyncJavaScript` 包出來的腳本本身回傳 Promise，WKWebView 的
    // `evaluateJavaScript` 立刻報「不支援的結果型別」，continuation 先以錯誤結束 → MissingResult）。
    // 同步 XHR 不能設 responseType，所以用 `x-user-defined` 字元集讓 responseText 的每個字元
    // 低 8 位元就是原始位元組。
    fn images(&self, urls: &[String]) -> Result<Vec<ImageResult>> {
        let urls_literal = serde_json::to_string(urls).map_err(|_| error!("Invalid url"))?;

        let js = format!(
            "(function(){{return JSON.stringify({}.map(function(u){{try{{var x=new XMLHttpRequest();\
             x.open('GET',u,false);x.overrideMimeType('text/plain; charset=x-user-defined');x.send();\
             if(x.status!==200)return {{e:'HTTP '+x.status+' cf-mitigated='+x.getResponseHeader('cf-mitigated')}};\
             var r=x.responseText,c=[],s='';for(var i=0;i<r.length;i++){{c.push(r.charCodeAt(i)&255);\
             if(c.length===32768){{s+=String.fromCharCode.apply(null,c);c=[];}}}}\
             s+=String.fromCharCode.apply(null,c);\
             return {{t:(x.getResponseHeader('content-type')||'image/jpeg').split(';')[0],d:btoa(s)}};\
             }}catch(e){{return {{e:String(e)}};}}}}));}})()",
            urls_literal
        );

        let raw = self.webview.eval(&js)?;

        serde_json::from_str(&raw).map_err(|_| error!("WebView 圖片請求失敗：{}", truncate(&raw, 200)))
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

// 用 `cf-mitigated` header 判斷，而不是比對 body 的「Just a moment」字串：前者是 Cloudflare
// 平台的標準 header，不會隨挑戰版本改措辭。403/503 都算，跟 host app 的判斷一致。
fn is_cf_challenge(status_code: i32, cf_mitigated: Option<&str>) -> bool {
    matches!(status_code, 403 | 503)
        && cf_mitigated.is_some_and(|value| value.eq_ignore_ascii_case("challenge"))
}
