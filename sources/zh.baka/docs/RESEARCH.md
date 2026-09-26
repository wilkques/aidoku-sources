# bakamh（巴卡漫畫）Cloudflare 解法筆記

> **目前解法總結（2026-09-26，已實機驗證：列表、封面、詳情、章節圖片全部正常）**
>
> **問題**：bakamh.com／.ru、baka3.me 全站都在 Cloudflare **interactive** 挑戰後面（`cType: 'interactive'`，
> 必須真人點擊）。頁面和 `/wp-content/uploads/` 的封面都一樣，app 用 URLSession 發的請求一律被擋。
>
> **解法**（跟 `zh.hip/docs/RESEARCH.md` 第十節同一套原理）：
> 1. **使用者手動驗證**：來源設定裡有一顆「Cloudflare 驗證（目前網域）」按鈕，由 `DynamicSettings`
>    動態產生，網址跟著目前選的網域走。它開的視窗和來源的 `WebView` 共用同一個 per-source cookie store，
>    所以使用者在這裡解出來的 `cf_clearance` 會存進來源自己的 store。
> 2. **頁面**：`src/fetch.rs::Web` 用 `load_html_blocking` 把空白文件的網址設成目前的網域，再用 `eval`
>    發同源同步 XHR 抓 HTML。遇到 CF 挑戰時自動重試 3 次、每次間隔 1 秒，都失敗才提示使用者去設定頁。
> 3. **封面**：列表抓完後，用同一個 WebView 依序抓這一頁的封面，轉成 `data:` 網址直接放進 `Manga.cover`。
> 4. **章節圖片**：`PageImageProcessor`。app 直接下載失敗時，閱讀器會把請求交給來源，來源改從 WebView
>    重新抓圖，再用 `ImageRef::new` 交回去。
>
> **使用者操作**：出現「Cloudflare 驗證已失效」時，到 瀏覽 → 巴卡漫畫 → 齒輪 →「Cloudflare 驗證（網域）」
> 完成驗證，關閉視窗後再重試。換網域後要重新打開設定頁、再驗證一次。
>
> **需求**：Aidoku 0.9（對應 aidoku-rs `e1320b0`）、iOS 17 以上（`.forSource` store 才是持久化的）。

---

## 一、為什麼這次行得通（推翻 2026-09-21 的「無解」結論）

舊的調查（25 輪實機測試）結論是「無解」，最強的證據是：「使用者在登入視窗完成驗證後，背景 WebView
仍然拿到驗證頁」。zh.hip 用同一套原理成功後，回頭比對，推測當時的實驗被干擾了：

- 當時 settings 預設送桌機 Chrome 145 的 UA。如果 WebView 導覽時帶了這個 header，而登入視窗用的是
  WKWebView 預設 UA，`cf_clearance` 綁定的 UA 就對不上，驗證一定被拒。
- 這次的寫法完全不設定 UA：`load_html`（不發網路請求）+ XHR，用的就是 WKWebView 自己的 UA，跟登入視窗一致。
- 那段舊實驗的程式碼沒有 commit 進 git，無法百分之百確認，但這次的實機結果已經足以推翻「無解」。

共用 cookie store 的根據（Aidoku 原始碼）：

| 元件 | 檔案 | data store | UA |
|---|---|---|---|
| 來源的 `imports::js::WebView` | `AidokuRunner/.../Utilities/WebViewHandler.swift` | `.forSource(key: id)` | 未設定 |
| 設定頁 `login` / `web` 視窗 | `Aidoku/App/Common/Settings/WebView.swift` | **同一個** `.forSource(key:)` | 未設定 |

---

## 二、實作細節與踩過的坑

### 2.1 驗證按鈕：一顆、跟著目前網域走

`source.json` 開啟了 `allowsBaseUrlSelect`，使用者可以在三個網域之間切換，而 clearance 是**每個網域各自獨立**的。

- **`urlKey` 不能用**：web 登入視窗只讀寫死的 `url`，`urlKey` 只有 OAuth 會用（`SettingView.swift` 的
  `loginWebSheetView` 對照 `handleOAuthLogin`）。這也是舊記錄裡「`urlKey` 會開出空白灰色視窗」的真正原因。
- **`requires: "url==https://..."` 可以用，但只能讓按鈕變灰、不能隱藏**：三顆按鈕都會顯示。
- **最後採用 `DynamicSettings`**（`settings.rs::get_cf_settings`）：app 每次打開來源設定頁都會重新呼叫
  `get_settings`，所以按鈕的網址可以用當下的 base URL 填入。按鈕的 key 是 `cfVerify_<host>`，「已驗證」
  狀態每個網域分開記。靜態的 `res/settings.json` 已刪除。
- **網址選單也改成動態產生**：`allowsBaseUrlSelect` 產生的內建選單只有 `refreshes: ["content"]`
  （AidokuRunner `Source.swift`），切換網址後設定頁不會收到 `refresh-settings`，驗證 / 登入按鈕
  還停在舊網址。現在拿掉 `allowsBaseUrlSelect`，在 `get_cf_settings` 自己出 key 同樣是 `url` 的
  `SelectSetting`，`refreshes` 加上 `"settings"`，切換後設定頁會立刻重新呼叫 `get_settings`。

### 2.2 設定頁「解碼錯誤」（兩個獨立的原因）

動態設定是用 **postcard** 傳給 app 的，postcard 按欄位**順序**解碼，不看欄位名稱：

1. **SDK 比 app 舊**：workspace 原本鎖在 aidoku-rs `a0624d4`，它的 `LoginSetting` 少了最後一個欄位
   `clear_cookies_on_log_out`，app 讀到最後資料不夠 → 解碼錯誤。已把整個 workspace 升到 `e1320b0`
   （對應 Aidoku 0.9）。
2. **Rust `bool` 對上 Swift `Bool?`**：`clear_cookies_on_log_out` 在 Rust 端是 `bool`，app 端是 `Bool?`。
   設成 `true`（`0x01`）時，app 會把它當成 Option 的「有值」標記，再多讀一個位元組 → 解碼錯誤。
   設成 `false`（`0x00`）剛好等於「沒有值」才解得開。所以**必須是 `false`**，代價是「清除 Cloudflare 驗證」
   只會重設按鈕狀態，不會真的刪 cookie。`pkce`、`use_email` 也是同樣的情況。

靜態的 `res/settings.json` 走 JSON 解碼，缺欄位沒關係，所以 zh.hip 沒遇到這些問題。

### 2.3 封面：`data:` 網址

- 封面在 `bakamh.com/wp-content/uploads/...`，curl 回 `403` + `cf-mitigated: challenge`。
- 封面載入失敗時，app **不會**回頭呼叫來源（`MangaGridCell` 只走 `CoverRecovery`；`CoverImageProcessor`
  只在載入成功時才執行），所以只能在抓列表時就先把圖抓好。
- `Web::list` → `inline_covers`：依序抓這一頁的封面，轉成 `data:<type>;base64,...`。某張失敗就保留原網址，
  並在 log 印出 `[baka] covers inlined X/Y, first error: ...`。

### 2.4 WebView 的 `eval_async` 在這版 Aidoku 不能用

第一版用 `eval_async` + `fetch` + `Promise.all` 平行抓圖，結果回傳 `JsError(MissingResult)`。原因在 host 端：

```swift
// WebViewHandler.evaluateAsyncJavaScript
let wrappedScript = "(async () => { ... })();"   // 這個運算式的值是 Promise
webView.evaluateJavaScript(wrappedScript) { _, error in
    if let error { continuation.resume(throwing: error) }  // WKWebView 不支援回傳 Promise → 先走到這裡
}
```

改用**同步 XHR**。同步 XHR 不能設 `responseType`，所以用 `overrideMimeType('text/plain; charset=x-user-defined')`，
讓 `responseText` 每個字元的低 8 位元就是原始位元組（`charCodeAt(i) & 255`），再用 `btoa` 轉成 base64。
已用 node 模擬驗證過，還原出來的位元組跟原圖完全一致。

### 2.5 章節圖片

**閱讀頁改版**：每一頁的結構變成

```html
<div class="page-break mkjp-slot" data-index="N">
  <img id="image-N" class="wp-manga-chapter-img mkjp-img" data-src="https://t1.bakamh.de/...jpg">
  <noscript><img src="(同一張圖)" class="mkjp-noscript"></noscript>
</div>
```

- 網址屬性從 `data-manga-src` 改成 `data-src`。舊程式因此抓到 0 張。
- 用 `.reading-content img` 會連 `<noscript>` 的備用圖一起抓到，每頁出現兩次，看起來像「順序怪異」。
- 現在只抓 `img[id^=image-]`（noscript 那張沒有 id），依 `N` 排序並去掉重複網址。`image_url()` 兩種屬性都支援。

**圖片主機 `t1.bakamh.de`**：curl 回 `403`，但**沒有** `cf-mitigated`，而且有 `Access-Control-Allow-Origin: *`。
看起來是防盜連（檢查 Referer），不是 CF 挑戰。app 直接下載失敗後，閱讀器會呼叫 `process_page_image`
（`ReaderPageView.swift` / `ReaderWebtoonPageNode.swift` 的 `processWithoutImage`，只在 `.dataLoadingFailed`
等錯誤時觸發），來源再從 `bakamh.com` origin 的 WebView 發 XHR 抓回來，自然帶上 Referer 和 Origin。
**`PageImageProcessor` 要保留。**

閱讀器是用 `try?` 呼叫處理器的，錯誤不會顯示在畫面上，所以失敗時會印 `[baka] page image failed: ...` 到 log。

### 2.6 帳號登入（限登入的漫畫）

部分漫畫要登入才能看。網站的登入是首頁上的 JS 彈窗，沒有獨立的登入頁。

- 設定頁「帳號」群組裡的「登入帳號（網域）」按鈕也是 `login` / `web`，打開首頁，使用者用網站自己的登入彈窗
  登入。web 登入視窗是完整的 WKWebView，所以 JS 可以正常運作。
- WordPress 的登入 cookie 會存進同一個 per-source store，`Web` 的 XHR 會自動帶上，來源程式完全不需要碰帳號密碼。
- 按鈕的 key 是 `account_<host>`，登入狀態每個網域分開記（cookie 綁網域）。
- 限登入的章節在未登入時沒有任何 `image-N`，`chapter()` 會直接報錯，提示去設定頁登入，不會回傳空白章節。
- 限制：`clear_cookies_on_log_out` 必須是 `false`（見 2.2），所以按鈕的「重設登入狀態」不會真的登出。
  要登出的話，打開視窗用網站自己的登出功能。
- 沒有用 basic（帳密）登入：那得自己從 WebView 送出 WordPress 的登入請求、處理 nonce，網站一改版就容易壞。

2026-09-26 實機驗證：登入後，限登入的章節可以正常閱讀。

### 2.7 其他

- 站方的 `title` 屬性被重複跳脫（HTML 裡是 `&amp;amp;`），解析後還留著 `&amp;`，用 `decode_entities` 再解一層。
- 首頁四個分類共用同一個 `Web`，依序抓、遇到錯誤就停，被擋時最多只等約 2 秒。

### 2.8 換章節卡住沒提示：錯誤改成文字頁（2026-09-26，已實機驗證）

Aidoku 閱讀器用 `try?` 呼叫 `getPageList`（`ReaderPagedViewModel.getPages`），source 的錯誤訊息被吞掉、
當成 0 頁：直接開章節只跳 app 的通用「無法載入章節」；捲軸模式捲到章末接下一章（`appendNextChapter`）
更是 0 頁就直接 `return`，畫面卡住、完全沒提示。

`get_page_list` 失敗時改回傳一頁 `PageContent::Text`（閱讀器以 Markdown 顯示），內容是原本的錯誤訊息
（例如「Cloudflare 驗證已失效…」）加上「處理完後請關閉閱讀器再重新開啟這一章」。代價：失敗時下載章節
會「成功」但只存到這一頁；閱讀器會快取這一章，所以驗證後要重開。

source 端沒辦法自動打開設定頁的驗證視窗：SDK 沒有任何顯示 UI 的 API（`WebView` 是背景用），
唯一會自動跳出的是 app 的 CloudflareHandler 彈窗，但它存 `.default()` store、解完走 URLSession，沒用。

---

## 三、不要再嘗試的方向

- 用 URLSession（`Request::send()`）帶 cookie：CF 只信任解驗證的 WKWebView（hip 9.4.2）。
- 在 XHR 或 WebView 導覽請求上自訂 User-Agent：`cf_clearance` 綁定 UA。
- 讓看不見的 WebView 自己過 interactive 挑戰：必須真人點擊。
- login 設定用 `urlKey`：web 登入視窗不支援。
- `clear_cookies_on_log_out: true`：會造成設定頁解碼錯誤（見 2.2）。
- `WebView::eval_async`：在這版 Aidoku 拿不到結果（見 2.4）。
- 用 `.reading-content img` 抓章節圖：會連 `<noscript>` 一起抓（見 2.5）。

## 四、診斷用的 log

| log | 何時出現 | 代表 |
|---|---|---|
| `[baka] covers inlined X/Y, first error: ...` | 列表有封面沒抓到 | 看錯誤：`HTTP 403 cf-mitigated=challenge` 代表驗證失效 |
| `[baka] cover fetch failed: ...` | 整批封面抓取失敗 | WebView／eval 本身出錯 |
| `[baka] page image failed: ...` | 章節圖片補抓失敗 | 閱讀器畫面上看不到這個錯誤，只能看 log |
