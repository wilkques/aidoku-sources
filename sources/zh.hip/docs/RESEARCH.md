# hipmh.com（嬉皮漫畫）來源重寫筆記

## 背景

`sources/zh.hip` 是從別的來源（`zh.mxs`）複製貼上開始的，程式碼裡到處殘留舊站的選擇器、舊的 `Mxs` 命名、跟現在這個站完全對不上的資料結構。站方本身也重構過（Astro SSR + Tailwind），舊版模板（`.mh-item`、`.banner_detail_form`、`#detail-list-select` 等）在新站完全不存在。這份筆記記錄逆向出來的實際站台結構，供之後維護/除錯參考。

網域一覽：

| 網域 | 用途 |
|---|---|
| `m.hipmh.com` | 主站：列表、搜尋、漫畫詳情頁 |
| `hipapi1.s3file.top` | API：篩選/分類列表、章節列表、章節圖片 |
| `reader.hipmh.top` | **獨立網域**，實際的閱讀器頁面 |
| `cover.s3imgs.top` | 封面圖 CDN |
| `hip-tx-1.s3imgs.top`（line1）/ 其他 line2 鏡像 | 章節內頁圖片 CDN |

---

## 一、Filters / URL 設計

`res/filters.json` 只有**一個** `"题材"` select，17 個選項混雜了排行/類型/地區/狀態，且**沒有 `ids`**，所以 `FilterValue::Select.value` 拿到的是選項的**中文字面值**（例如 `"人氣榜"`、`"韓漫"`），不是 index 也不是 id。

`url.rs` 用 `(label, 對應值)` 對照表直接查，取代逐一 index 猜測：

```rust
const PATH_OPTIONS: &[(&str, &str)] = &[("人氣榜", "popularity"), ("本周熱門", "weekly")];
const GENRE_OPTIONS: &[(&str, i32)] = &[("系統", 67), ("玄幻", 27), ...]; // 10 個
const CATEGORY_OPTIONS: &[(&str, i32)] = &[("韓漫", 1), ("陸漫", 2), ("日漫", 3)];
const STATUS_OPTIONS: &[(&str, &str)] = &[("連載中", "ongoing"), ("已完結", "completed")];
```

對應到單一 `Url::Filter { kind: FilterKind, page }`，`FilterKind` 是 `Path/Genre/Category/Status` 四選一的 enum（原本是四個各自獨立的欄位＋一串 `if let`，容易同時有值、邏輯矛盾，改成 enum 後同一時間只可能是其中一種）。

沒有任何篩選條件時，`Url::filters()` fallback 到 `FilterKind::Path("popularity")`（見對話決定，取代原本的 `ListType("booklist")`）。

---

## 二、列表頁 `https://m.hipmh.com/popularity`

Fixture：[`popularity.html`](./popularity.html)（18 筆真實資料）。

```html
<div class="manga-section-item">
  <a href="/works/bToyMzQ3NQ-yi-ren-zhi-xia-tx-531490-17793" class="manga-card-link" aria-label="一人之下">
    <div class="manga-card">
      <div class="manga-card-image-wrapper">
        <img src="https://cover.s3imgs.top/.../....webp" alt="一人之下" class="manga-card-image">
        <div class="manga-card-rank-badge"><div class="rank-badge rank-badge-1">1</div></div>
        <div class="manga-card-content"><h3 class="manga-card-title">一人之下</h3></div>
      </div>
    </div>
  </a>
</div>
```

選擇器：

- 項目：`.manga-card-link`（`<a>` 本身，href 直接可用）
- id/key：`href` 最後一段（`/works/{slug}`，例如 `bToyMzQ3NQ-yi-ren-zhi-xia-tx-531490-17793`，整串都是 key，不能只取 `-` 前面那段）
- 封面：`img[src]`（沒有 `background-image` style 包裝了）
- 標題：`.manga-card-title`，備援 `[aria-label]`

**分頁真實存在**：`/popularity?page=2`、`?page=3`...（`pagination-next` 有 `aria-disabled` 可判斷是否最後一頁，目前用 `!entries.is_empty()` 簡化判斷）。

---

## 三、詳情頁 `https://m.hipmh.com/works/{key}`

Fixture：[`detail.html`](./detail.html)（一人之下）。

`Url::book` 原本是 `/book/{id}`——**這是錯的**。實測 `/book/{id}` 回 `HTTP 200` 但 `Content-Length: 0`（Cloudflare 快取住的死路由），`/works/{id}` 才是真正的頁面。已修正。

關鍵選擇器：

| 欄位 | 選擇器 |
|---|---|
| 標題／封面 | 頂層 `[data-manga-title]` 容器的 `data-manga-title` / `data-cover-url` 屬性 |
| 作者 | `a[href^="/author/"]` |
| 標籤/類型 | `a[href^="/genre/"]` |
| 簡介 | `.whitespace-pre-line`（整頁唯一一個符合的元素） |
| 狀態 | 比對 `a[title="連載中"]` / `a[title="已完結"]`（繁體，跟 filters.json 用字一致；用 `title` 屬性比用 `href` 路徑穩，因為沒驗證過「已完結」時 href 實際長怎樣） |

`data-manga-title` 容器上也有 `data-manga-id`（數字 id，例如 `23475`）——章節 API 會用到（見下）。

---

## 四、章節列表 API（不在靜態 HTML 裡）

詳情頁本身**不含章節清單**，前端另外打 API。從 `chapters-manager.*.js` 找到：

```
GET https://hipapi1.s3file.top/v1/manga/chapters?mid={mid}&page={page}&per_page={per_page}&order=asc|desc
```

- `mid`：從 detail 頁 `#chapters-config[data-mid]` 讀（例如 `bToyMzQ3NQ`，跟 works 頁的完整 slug 不同，是它的前綴）
- `per_page` 實測**上限固定 50**，request 帶更大值會被站方直接 clamp
- response：

```json
{
  "code": 200,
  "data": {
    "total": 814,
    "total_pages": 17,
    "items": [
      {
        "hid": "bToyMzQ3NS1jOjU5NTk1-MjM0NzU6MS4wMA",
        "chapter_number": 1,
        "title": "1.姐姐1",
        "cover_image_url": "/tx/chapter/23475_1_1-jie-jie-1-06afd75b.webp",
        "created_at": "...", "updated_at": "2026-07-16T22:11:29.64139Z"
      }
    ]
  }
}
```

用 `order=asc` 直接拿正序（chapter_number 從 1 開始），不用像原本選擇器版本那樣抓完再 `.reverse()`。`updated_at` 取 `T` 前面的日期部分（`yyyy-MM-dd`）餵給 `aidoku::imports::std::parse_date` 當 `date_uploaded`。

Fixture：[`chapter_images.json`](./chapter_images.json) 其實是章節**圖片** API 的回應（見下一節），章節**列表** API 目前沒有另外存 fixture。

---

## 五、閱讀器網域 + 圖片解密（最複雜的部分）

### 5.1 `Url::chapter` 原本也指錯網域

最早以為 `{base_url}/chapter/{id}`（也就是 `m.hipmh.com/chapter/{id}`）是對的，因為 curl 回 200。**這是誤判**——那個 200 其實是首頁內容（`<title>首頁</title>`），因為只檢查了狀態碼跟大小，沒看內容。

真正的重導向鏈：

```
m.hipmh.com/chapter/go?hid={hid}&m={mangaId}
  → reader.hipmh.top/chapter/go?hid={hid}&m={mangaId}   （跨網域 JS 重導向，data-host 屬性指定）
  → reader.hipmh.top/chapter/{hid}                       （最終頁面）
```

`Url::chapter` 已修正成打 `settings::get_reader_url()`（`https://reader.hipmh.top`），不是 `base_url`。

### 5.2 頁面本身不含明文圖片網址

閱讀器頁的 `#chapcontent` 元素上有一堆 `data-*` 屬性，其中：

- `data-api-hid`：**跟章節列表 API 給的 `hid` 不是同一個值**，是另一組專門給圖片 API 用的 hid
- `data-api-base-url` = `https://hipapi1.s3file.top`
- `data-chapter-img-base`（line1 預設）= `https://hip-tx-1.s3imgs.top`（還有 line2 備援鏡像，站方前端會存 localStorage 記使用者選的線路，我們固定用 line1／頁面當下給的值）

圖片 API：

```
GET https://hipapi1.s3file.top/v2/chapter?hid={api_hid}
```

回應的 `data.images` 是**一整串打亂/編碼過的字串**，不是圖片網址陣列。前端用 `/assets/runtime/chapter-decoder.js`（javascript-obfuscator 重度混淆，字串陣列旋轉 + 控制流平坦化）解開。

### 5.3 逆向解密演算法的方法

沒辦法直接讀懂混淆碼，改用「黑盒插樁」：

1. 找到 `node.exe`（這台機器裝在 `F:/works/node/`，不在 PATH，路徑要用 Windows 格式 `C:/Users/...` 而不是 Git Bash 的 `/tmp/...`，兩邊對不上會抓錯檔案）。
2. `global.window = {}` 後直接 `require()` 混淆檔，讓它自己把 `window.__cimg.r` 掛上去，再餵真實 API 回應進去，拿到「正確答案」（解出來的圖片路徑陣列）。
3. 用字串取代法，在混淆原始碼裡的固定錨點（例如某個變數宣告後面）插入 `console.log`，重新 `require()` 執行過的版本，把中間變數（切割用的 offset、alphabet 等）一個一個挖出來，而不是硬看混淆過的三元運算式。
4. 反覆對照真實輸入/輸出，直到公式跟真實答案完全吻合。

### 5.4 還原出來的演算法

```
輸入字串（例如 "qM9...Z7"）：
  1. 去掉頭尾標記 "qM9"（prefix）跟 "Z7"（suffix），剩下 core
  2. core 再扣掉內部兩個標記 "Vx"（2 字）、"pL0"（3 字）的長度，剩下 rem
  3. tail_len  = floor(rem / 3)
     remainder = rem - tail_len
     head_len  = floor(remainder / 2)
     junk_len  = remainder - head_len
  4. core 依序切成： [head][*"Vx"*][junk][*"pL0"*][tail]
     驗證：中間兩個標記字面值要對得上，且 tail.len() == tail_len，否則視為格式錯誤
  5. 重組成 combined = tail + head + junk（丟掉兩個標記本身）
  6. 每 7 字一組切 chunk，偶數組（0-based）維持原樣、奇數組整組反轉
  7. 用自訂字母表
       "_-9876543210abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
     當 base64 字母表直接解碼成 bytes
     （這個字母表跟標準 base64url 字母表 "A-Za-z0-9-_" 的差別只是「順序被打亂」，
     但每個字元對應的 6-bit 數值跟標準字母表在同一個 index 位置的意義相同，
     所以可以完全跳過「先轉成標準 base64 字串」這一步，直接用自訂字母表的 index 當 6-bit 值）
  8. 6-bit 值依序拼成 bytes，UTF-8 decode 成字串
  9. 該字串是 JSON 陣列，內容是圖片的相對路徑，例如：
     ["/i/zOWt2hzKWm3pS8DS0BeY5s1H8cUvKJ-.../dHg6MjM0NzU6MToxOjMzMjAx_1.36n7im.webp", ...]
  10. 每個路徑前面接上 data-chapter-img-base（例如 https://hip-tx-1.s3imgs.top）就是完整圖片網址
```

Rust 實作在 `src/decoder.rs`（`decode_chapter_images`），純函式、無外部 crate（base64 解碼手刻，因為字母表本來就是自訂的，直接查表比套標準 base64 crate 再額外做字元對照更省事）。

Fixture：[`chapter_images.json`](./chapter_images.json) 是一人之下第 1 話的真實 API 回應，離線測試 `decode_chapter_images` 就是拿這包資料驗證，第一筆／最後一筆（共 18 張圖）的完整路徑都比對過。

**注意**：`qM9`/`Z7`/`Vx`/`pL0` 這四個標記字串、字母表、chunk size（7）全部是這次反混淆當下（`chapter-decoder.js` 某個 hash 版本）觀察到的常數。如果站方哪天換了新的 decoder 檔案（檔名 hash 會變），這些常數很可能也會跟著換，屆時要重新跑一次 5.3 的插樁流程。

---

## 六、Home 首頁

參考 `zh.jmtt` 的設計模式：`get_home()` 先用 `send_partial_result` 丟一份 skeleton layout，背景平行 `Fetch::get(...).send()` 六個分類，非空才塞進最終 `HomeLayout`；`ListingProvider::get_manga_list` 用同一組 `listing.id` 對回同一個 `Url::Filter`，讓每個分類點進去可以翻頁瀏覽。

六個分類直接對應 filters.json 選項，不用像 jmtt 拼假的 `FilterValue::Select` 再繞回 `Url::filters()` 反查（因為 `Url::Filter{ kind, page }` 本身就是 `pub`，可以直接建構）：

| Listing id | 名稱 | FilterKind |
|---|---|---|
| `popularity` | 人氣榜 | `Path("popularity")` |
| `weekly` | 本周熱門 | `Path("weekly")` |
| `korean` | 韓漫 | `Category(1)` |
| `mainland` | 陸漫 | `Category(2)` |
| `japanese` | 日漫 | `Category(3)` |
| `ongoing` | 連載中 | `Status("ongoing")` |

---

## 六之一、章節圖片清單裡的誘餌（decoy）頁

**症狀**：aidoku 上某些章節捲動到某個位置會出現一大塊黑色空白（使用者截圖：「一人之下」
第58話，捲到 47/124 頁時卡在一片黑），但網站自己的閱讀器 `reader.hipmh.top` 看同一話
是連續無縫的。

**根因**：`decode_chapter_images` 解出來的路徑陣列，偶爾會在**同一個頁碼相鄰出現兩筆**，
例如：

```
.../..._46.2gi6me.webp
.../..._46.w5v1n8.webp
```

實測其中一筆的回應是 `HTTP 200`、`Content-Type: image/png`，內容只是幾十 bytes 的 1x1
透明圖（`Cache-Control: no-store`，沒有 `Etag`/`Last-Modified`，明顯是即時生出來的假回應，
不是真的靜態檔案）；另一筆才是真正的 webp 內頁（有 `Etag`、10 年 `max-age` 的 CDN 快取）。
這應該是站方防爬蟲塞的誘餌頁。

**踩過的坑**：一開始以為「誘餌一定排在真圖前面」（在「一人之下」兩個不同章節各測一次都
是誘餌在前、真圖在後），照這個假設寫了「同頁碼時只保留較後面那筆」的無網路版本。結果用
使用者實際回報的章節（`https://reader.hipmh.top/chapter/bToyODU1MS1jOjEyODQ2MQ-Mjg1NTE6NTguMDA`，
另一本書）重新驗證，發現**同一組裡誘餌反而排在後面**——順序完全不可靠，不能只憑陣列位置
判斷，兩種順序都會發生。

**修法**（`html.rs::resolve_decoy_duplicates`）：解碼完先找出「相鄰、頁碼相同」的配對，
只對這些配對額外打 `HEAD` 請求（用 `Request::send_all` 平行送出），看回應的
`Content-Type` 是不是 `image/png` 來判斷哪一筆是誘餌並踢掉。這種相鄰重複很少見（實測
124 頁的章節只出現 1 組），多打的 HEAD 請求數量可忽略；如果 HEAD 請求失敗判斷不出來，
兩筆都保留，不冒著踢掉真內頁的風險。`decoder.rs` 裡的 `decode_chapter_images` 本身維持
單純解碼、不做這個過濾（純函式，離線 fixture 測試不受影響），頁碼取出邏輯抽成
`decoder::page_number_key` 給 `html.rs` 共用。

---

## 七、還沒驗證 / 已知風險

- **章節圖片解密**：本機 `aidoku-test-runner` 因為 Windows 工具鏈缺 `dlltool.exe` 跑不起來（`cargo install aidoku-test-runner` 會失敗），沒辦法在 wasm 環境裡實際端到端測過；目前的信心來自「用 Node 直接執行混淆碼＋比對 Rust 手刻邏輯的中間值」，邏輯上應該等價，但還沒有手機上實機確認章節圖片能正常顯示（進行中）。
- **`cargo test` 對 `m.hipmh.com` 的網路請求會失敗**（`RequestError`），研判是 `reqwest`（test-runner 用的 client）的 TLS 指紹被 Cloudflare 擋掉，`curl`／真機不受影響；純選擇器/解碼邏輯已經全部改用不連網的 fixture 離線測試繞過這個限制，見 `src/test.rs`（目前整份被註解，需要用時解開特定測試即可）。
- `get_page_list` 之外，`Url::book`/`Url::chapter` 已修正，但 home.rs 舊版 `.mh-item`/`.mh-item-tip` 選擇器邏輯（服務首頁用）已經整段刪除，改直接共用 `GenManga::list()`。
- 章節圖片 CDN 有 line1/line2 兩條線路可以切換（站方前端存 localStorage），目前固定吃頁面當下給的 `data-chapter-img-base`（line1 預設），沒有實作切線路的容錯機制。
- ~~`home.rs::get_home()` 平行 `.send()` 六個分類的寫法，只要其中一個分類打到未快取過的 URL 被 CF 擋下，`korean?`/`mainland?`/... 的寫法會讓整個 `get_home()` 失敗（不是該分類單獨消失）。~~ **已於 2026-09-25 修正**（見 9.3）：改成個別分類失敗只讓該分類消失，六個全滅時才把第一個錯誤傳出去——否則使用者只會看到一片空白首頁。六個分類裡有四個打的是 CF 後面的 `hipapi1`，在第九節的診斷下被擋是常態，所以這個容錯是必要的。

---

## 八、章節列表／章節圖片 API 被 Cloudflare 間歇擋下（2026-09-21）

### 8.1 症狀

使用者回報：「取得章節列表第一次後再取得就會失敗」，追問後確認是**章節內頁圖片**——看完一話換下一話，章節圖片抓取失敗（`get_page_list`／`GenManga::chapter()`）。

### 8.2 診斷

一開始用 curl 連續打同一顆 `hipapi1.s3file.top` 的 `v1/manga/chapters`／`v2/chapter` API 30+ 次都是 `200`，一度以為站方沒有保護。後來換一個沒打過的查詢組合（`per_page=3&order=asc`）立刻拿到 `403` + `Just a moment...`（Cloudflare managed challenge，`cType: 'managed'`）的 HTML，而且**觸發之後連原本一直正常的那顆 URL 也一起被擋**。

關鍵：之前一直正常的請求，其實是因為那個確切的 URL（例如熱門漫畫 `page=1&per_page=50&order=desc`）已經被 Cloudflare edge 快取（`cf-cache-status: HIT`），直接吐快取內容，沒有真的打到會做 bot 檢查的 origin。只要 URL 沒被快取過（冷門章節的 `v2/chapter?hid=...`、不常見的分頁/排序組合），就會真的觸發 managed challenge，回傳 HTML 而不是 JSON，`json_owned()`／`.html()` 選擇器就會失敗或找不到預期節點。

`resolve_decoy_duplicates`（六之一）對同網域多打的 HEAD request，也會增加短時間內的請求量，讓保護更容易被觸發。

`m.hipmh.com`／`reader.hipmh.top` 兩個 HTML 網域背後同樣掛了 Cloudflare（`Server: cloudflare`），只是實測當下常見頁面（熱門漫畫詳情頁、其第一話閱讀頁）都是長 TTL 的快取命中，還沒實際重現過這兩個網域的 challenge 頁，但架構上風險相同，故一併套用下面的修法。

### 8.3 修法：direct-first + 背景 WebView fallback

沿用 `zh.happy`（`sources/zh.happy/docs/RESEARCH.md` 步驟十七起）已在真機驗證過的模式，實作在 `src/fetch.rs`：

1. `fetch_text(url)`：先直接 `Request::string()`；回應內容比對 `"Just a moment"` / `"_cf_chl_opt"` 判斷是不是驗證頁（不是像 happy 那樣用 `<!` 開頭，因為 hip 這裡同時要處理真正的 HTML 頁，`<!DOCTYPE` 開頭的合法頁面也會誤判）。
2. 判斷是驗證頁才退回 `fetch_text_webview(url)`：`WebView::new()` 直接 `load_blocking` 同一個 URL，讓 CF 的自動 JS 挑戰在背景 WebView 內跑完、把 `cf_clearance` 存進與原生 `Request` 共用的 cookie store；`sleep(2)` 等挑戰真正跑完（同 happy 步驟十七的教訓：`load` 事件比挑戰腳本執行還早）；接著用同源同步 XHR（`wv.eval`）重新打一次同一個 URL，拿 `x.responseText`。
3. 因為 hip 這邊要保護的每個端點本身就是可以直接導覽的 GET（章節列表/圖片 JSON API、書籍詳情頁、閱讀頁），不像 happy 的 POST 搜尋 API 需要另外一個「觸發頁」常數（`CF_TRIGGER_PATH`）——直接拿目標 URL 本身當觸發頁即可。
4. `fetch_json::<T>(url)` 疊在 `fetch_text` 上，`serde_json::from_str` 解析失敗時把回應內容（截斷 200 bytes，char boundary 安全）塞進錯誤訊息，方便之後看 log。
5. `fetch_html(url)` 疊在 `fetch_text` 上，改用 `Html::parse_with_url` 解析成 `Document`，讓既有的 CSS selector 程式碼（`GenManga::list/detail/chapters/chapter`）不用改。

套用範圍：章節列表 API（`html.rs::chapters`）、章節圖片 API（`html.rs::chapter`）、genre/category/status 篩選與搜尋 JSON（`json.rs::fetch_manga_list_json`/`fetch_search_json`）、以及全部四個 HTML 頁面請求（`lib.rs` 的搜尋/篩選列表、書籍詳情頁、閱讀頁）。唯一**沒有**套用的是 `home.rs::get_home()` 的六個平行 `.send()` 請求（見上方七、的補充）——WebView fallback 本質上是同步阻塞操作，套不進現有的平行 `Response` 陣列寫法，需要的話要另外設計。

### 8.4 還沒驗證（第一版）

跟第五節的圖片解密一樣，本機 `aidoku-test-runner` 跑不起來，`WebView`/`sleep` 這條路徑目前只有邏輯層面比照 `zh.happy` 已驗證過的寫法，還沒有手機實機確認 managed challenge 真的能在背景 WebView 內自動解掉（`zh.happy` 當初是驗證成功的案例，但 `zh.baka` 也有過「背景 WebView 過不了互動式驗證」的先例——如果 hip 這邊實測仍然失敗，要先確認擋下來的是 managed challenge 還是需要人手動點的互動式驗證，兩者修法完全不同，後者參考 `zh.baka` 的結論可能是無解）。

### 8.5 實機回報：WebView fallback 本身也持續被擋（2026-09-21，第一版有問題）

使用者在**章節更新**（`get_manga_update` → `GenManga::chapters()`）實測，貼出真實 log：

```
[hip] direct hit CF challenge (len=5932) -> trying webview: https://hipapi1.s3file.top/v1/manga/chapters?mid=...
Error: Message("webview hit CF challenge (len=6060): <!DOCTYPE html>...Just a moment...")
```

重點：不是直接請求被擋而已——**8.3 的背景 WebView fallback 本身也拿到同一張 `Just a moment` 驗證頁**，代表 `load_blocking` + `sleep(2)` + `wv.eval` XHR 這一套在真機上沒有真的把挑戰解掉，症狀正是使用者說的「一直要求重複驗證」。

研判兩個可能原因（兩個都改了，還沒個別驗證是哪個起作用）：

1. **User-Agent 訊號不一致**：`fetch_text_webview` 原本用 `Fetch::get(url)`（`src/settings.rs::get_user_agent()`，預設偽裝成桌機版 Chrome 135）當 `load_blocking` 的導覽請求。這個 UA 只會出現在那一次的 HTTP header 上，WebView 引擎實際執行 CF 挑戰腳本時，腳本讀到的 `navigator.userAgent`／TLS 指紋是裝置真實的值，跟 header 宣稱的桌機 Chrome 對不上。Cloudflare bot management 常見會比對這類訊號一致性，宣稱值跟實際環境不符，挑戰即使算完也可能不被採信。**修法**：`fetch_text_webview` 的導覽請求改用不帶自訂 `User-Agent` header 的裸 `Request::new(url, HttpMethod::Get)`，讓 WebView 全程用它自己真實的身份，不要用我們手動塞的桌機 UA 污染它。
2. **等待時間不夠**：原本只 `sleep(2)` 一次就 `eval`，若那一輪還沒解完就直接判定失敗。**修法**：改成同一個 `WebView` 內最多等 3 輪（`sleep(3)` 起手，之後每輪 `sleep(2)`），每輪都重新 `eval` 同一段 XHR，中間**不重新導覽**（挑戰腳本持續在背景那個已載入的頁面內跑，不需要也不應該重新整頁）。

兩個修法都已套用到 `src/fetch.rs::fetch_text_webview`，但**還沒有使用者實機的第二輪 log 驗證有沒有解決**。如果之後 log 顯示 webview fallback 還是持續拿到 `Just a moment`（不是逐漸變好，是每次都一樣），代表 8.4 提到的「其實是互動式驗證、不是自動 JS 挑戰」的可能性更大，屆時要換方向查證，不要再單純調參數。

### 8.6 決定性結論：hipapi1.s3file.top 的 JSON API 判定為無解，拿掉 WebView fallback（2026-09-21）

**三輪實機測試，三次都失敗，訊號完全一致**：

1. 第一版（`sleep(2)`、帶自訂 UA）→ 失敗
2. 第二版（不帶自訂 UA、最多重試 3 輪）→ 失敗，使用者回報「一直要求重複驗證」
3. 第三版（拿掉重試迴圈、單次 `sleep(3)`）→ 使用者貼 log，**還是同一張 `Just a moment` 驗證頁**（`webview hit CF challenge (len=5912)`）

三次調的是完全不同的變因（UA、等待時間、重試次數），結果卻**完全一樣**——這代表問題不在參數，背景 `WebView` 結構上就是解不開 `hipapi1.s3file.top` 這個 zone 的挑戰。

**跟 `zh.baka` 比對，判定是同一類問題**：查了 `zh.baka` 的完整記錄（memory `project-baka-cloudflare-blocked`，25 輪實機測試才決定性證實無解），核心事實是 `imports::js::WebView` 的官方文件寫死「This web view won't be displayed to the user. It is intended for use in the background.」——background WebView **沒有畫面**，如果 Cloudflare 把這個 zone 的挑戰升級成需要真人互動才能過（不只是自動跑一段 JS），背景 WebView 無論怎麼調 sleep/UA/重試次數，結構上都不可能有人去點那個看不到的畫面。三輪不同配置、同樣失敗的訊號，跟這個結論完全吻合。

**順帶查證：「App 自動彈出驗證視窗」不是 SDK 功能，source 端無法觸發也無法偵測**。對照 `aidoku-rs` 原始碼（`crates/lib/src/imports/net.rs`），`Request::string()`/`html()`/`json_owned()` 三個方法底層都呼叫同一個 `send()`，對 host app 來說是完全相同的請求，沒有任何欄位能區分「這是使用者要看的頁面」還是「背景資料抓取」；SDK 裡也沒有任何 Cloudflare 相關 API（唯一相關的 `WebLoginHandler` 只是登入流程的 cookie callback，語意上跟 CF 無關）。`zh.happy` 記錄裡提到的「App 內建 CF bypass dialog」是實機測試時觀察到的 host app 黑盒行為，不是這個開源 SDK 提供的功能，我們無法從 source 程式碼主動觸發或確認它有沒有出現——這也解釋了為什麼 hip 這邊從頭到尾沒有跳出任何驗證畫面。

**修法**：`src/fetch.rs::fetch_json`（JSON API 專用）**拿掉 WebView fallback**，直接請求失敗就快速回傳清楚的錯誤，不再嘗試背景 WebView：

- 繼續掛著一個已知永遠失敗的 fallback，只會讓每次失敗多打兩發請求（WebView 導覽 + eval XHR）到已經被 Cloudflare 判定可疑的網域，外加白等 3 秒——不只沒用，反而可能讓風險分數/封鎖狀態更難恢復。
- `fetch_html`（`m.hipmh.com`/`reader.hipmh.top` 的 HTML 頁）**保留** direct-first + WebView fallback：目前回報的失敗案例都集中在 `hipapi1.s3file.top` 的 JSON API，這兩個網域是不同的 Cloudflare zone，沒有證據顯示同樣的結論適用，先不動；如果之後這兩個網域也回報類似的持續失敗，比照這裡的結論拿掉 fallback。

**現狀**：`hipapi1.s3file.top` 一旦打到沒被 CDN 快取過的 URL（冷門章節、不常見的分頁/hid 組合），會直接失敗，沒有自動繞過的方法。使用者能感受到的行為：熱門漫畫、常被讀的章節通常正常（吃 CDN 快取）；冷門漫畫或换到没什么人看过的章節容易失敗。這是目前 `aidoku-rs` SDK 能力範圍內的已知限制，不是程式碼邏輯錯誤——除非之後有新的 SDK API（例如讀取/複用 host app 的 `cf_clearance`，或明確觸發互動式驗證的介面），否則不用再花時間調 WebView 參數。

### 8.7 查證：Aidoku-Community/sources 有沒有其他 source 解過同樣的問題（2026-09-21）

把官方社群 source 倉庫（`github.com/Aidoku-Community/sources`）clone 下來全文搜尋
`Cloudflare`/`cf_clearance`/`Just a moment`/`WebView` 等關鍵字，找到少數幾個相關案例：

- `en.mangakakalot`：測試檔案裡直接寫「listings 全部壞掉，因為 `/genre/all?filter=` 被
  Cloudflare 擋，但首頁還是正常」——**沒有修法，就是承認壞掉**。
- `multi.myreadingmanga`：頁面清單抓空時 `bail!` 提示「可能是網站掛了或出現 Cloudflare
  警告」——單純錯誤訊息，沒有嘗試繞過。
- `multi.mangadotnet`：genre 產生器是一支**開發時在本機手動跑**的 Python 腳本
  （`scripts/genre_generator.py`），要人先從自己瀏覽器複製 `cf_clearance` 貼進腳本裡再
  執行——這是建置期工具，不是 wasm source 執行期程式碼，不能套用在我們的情境。
- `en.comix`：**目前社群裡對 Cloudflare 處理最成熟的案例**，但結論跟我們一致——**沒有
  自動解過真正的 Cloudflare 挑戰**。它用 `response.status_code() == 403 &&
  response.get_header("cf-mitigated") == "challenge"` 判斷是不是真的被 CF 擋（見
  `sources/en.comix/src/web.rs`），比我們原本只比對 body 字串「Just a moment」更權威——
  `cf-mitigated` 是 Cloudflare 平台本身標準的 response header，不像 body 措辭可能隨挑戰
  版本改變。抓到之後**直接 `bail!` 一則講清楚的錯誤訊息**（「請清除來源快取並重啟 app」），
  不嘗試背景 WebView 自動解。它自己也用 `WebView` 執行 JS，但那是用來跑站方自己的
  obfuscated JS 解出圖片/API request 簽名，跟解 Cloudflare 挑戰是兩回事；它另外做了一個
  「Verify Captcha」設定按鈕（`"type": "login", "method": "web"`），但那個是解決 comix
  **自家的** WAF captcha（`captcha_required` JSON 欄位），程式碼裡明確把這個跟
  `cf-mitigated`（真正的 Cloudflare）分開處理成兩條路——也證實「登入視窗類的按鈕」不是
  拿來解真正 Cloudflare 挑戰用的。

**結論**：翻遍社群倉庫沒有找到任何 source 真的用程式碼自動解開 Cloudflare 的機器人挑戰；
`en.comix` 的作法（用 `cf-mitigated` header 精準判斷 + 清楚的錯誤訊息，不硬解）已經是
目前看到最成熟的處理方式，跟我們拿掉 WebView fallback 的方向一致。已採用它的偵測方式：
`fetch.rs::is_cf_challenge_response` 改用 `response.status_code() == 403 &&
get_header("cf-mitigated") == "challenge"` 當主要判斷依據（比原本單純比對 body 字串
「Just a moment」/`_cf_chl_opt` 更精準），字串比對留著當備援。

### 8.8 決定性根因：host app 原始碼證實——WebView fallback 結構上不可能有用（2026-09-21）

查了 host app 本身（`github.com/Aidoku/Aidoku`）跟它依賴的 WASM 執行環境
（`github.com/Aidoku/AidokuRunner`，注意這是**跟 `aidoku-rs` 不同的另一個 repo**——
`aidoku-rs` 只是 Rust 端 SDK，實際執行 wasm、串接 `net`/`js` import 的 Swift 端實作在
`AidokuRunner`）的原始碼，找到兩個決定性事實，糾正並補完 8.6 的結論：

**事實一：app 本身確實有一套自動化的 Cloudflare 處理機制，而且遠比我們自己土砲的
WebView fallback完整**——`Aidoku/Core/Sources/Cloudflare/CloudflareHandler.swift`。
`AidokuRunner.swift` 的 `InterpreterConfiguration.defaultConfig` 把它接進
`requestHandler`，是每一次 `net.send()`（也就是 Rust 端每個 `Request::send()`／
`.string()`／`.html()`／`.json_owned()`）的必經之路，source 端完全不用做任何事就會
自動套用：

1. 判斷是不是 CF 擋（`shouldHandle`）：`Server` header 是 `cloudflare`/`cloudflare-nginx`
   + 狀態碼 403/503 + body 用 SwiftSoup 解析出 `#challenge-error-title` 或
   `#challenge-error-text`（hip 的驗證頁裡確實有 `id="challenge-error-text"`，這個判斷
   對 hip 是會命中的）。
2. 建一個**跟原始請求 User-Agent 對齊**的隱形 `WKWebView`（`0x0` 大小），導覽同一個
   request，讓 CF 的自動 JS 挑戰有機會跑完。
3. 導覽完成後 3 秒、6 秒各檢查一次（`checkForCaptcha`，注意跟我們踩過的「load 事件後
   挑戰腳本才開始跑」是同一個教訓，app 自己也要多等），如果偵測到還卡在
   `cf-turnstile-response`／`#challenge-error-title`／`#challenge-error-text`／
   `document.title === "Just a moment..."` 任一種驗證頁特徵，就把**這個隱形 webview
   直接顯示成一個彈出視窗**（`showPopup`），讓真人手動過。
4. 偵測到 `WKWebsiteDataStore` 裡出現新的 `cf_clearance` cookie（且真的跟舊值不同，避免
   誤判舊快取），就把新 cookie 橋接進 `HTTPCookieStorage.shared`——這正是
   `URLSession.shared`（我們每個 `Request::send()` 底層用的）會讀的 cookie 儲存區。
5. 整體有 12 秒逾時；逾時、使用者關掉彈窗、或橋接完 cookie 後重打原始請求仍然被擋
   （`solveFailed`），就會把**原始被擋的回應**原封不動丟回給 source（也就是我們 Rust
   端看到的那張 `Just a moment` 頁）。

也就是說：**我們的 Rust 程式碼看到驗證頁的當下，代表 app 內建這一整套（含彈出視窗給
真人手動過）已經先跑過一輪且失敗了**，不是完全沒試過。這修正了 8.6 「source 端完全無法
觸發／偵測 app 的驗證彈窗」的說法——SDK（`aidoku-rs`）本身確實沒有相關 API 沒錯，但
host app 在更底層（`net` import 的實作）自動幫每個請求套用了，不需要 SDK 開放介面。

**事實二：我們自己在 Rust 端另開的 `imports::js::WebView`，天生用的是被隔離的
cookie 儲存區，就算解開挑戰也沒用**——`AidokuRunner/Sources/AidokuRunner/Utilities/
WebViewHandler.swift`：

```swift
init(id: String) {
    let config = WKWebViewConfiguration()
    config.websiteDataStore = .forSource(key: id)   // 每個 source 各自獨立的儲存區
    self.webView = WKWebView(frame: .zero, configuration: config)
    ...
}
```

`.forSource(key:)`（`AidokuRunner/Extensions/WKWebsiteDataStore.swift`）用
`WKWebsiteDataStore(forIdentifier: UUID(key: key))` 建立一個**每個 source 專屬、
互不相通**的持久化 cookie 儲存區。這跟事實一裡 `CloudflareHandler` 用的
`WKWebsiteDataStore.default()`，以及 `URLSession.shared` 讀的
`HTTPCookieStorage.shared`，是**三個完全不同、互不同步的儲存區**。

換句話說：就算我們自己在 `fetch_text_webview`（8.5～8.6 拿掉之前的版本）裡開的背景
`WebView` 真的把 Cloudflare 挑戰解開了，拿到的 `cf_clearance` 也只會寫進一個沒有任何
其他請求會去讀的隔離罐子——不會進 `HTTPCookieStorage.shared`，後續任何
`Request::send()` 都吃不到。這才是三輪調參數（UA／sleep／重試次數）在真機上全部失敗
的**真正結構性原因**，跟 Cloudflare 到底是不是真的把挑戰升級成互動式已經沒有關係了：
就算是最簡單的自動 JS 挑戰，我們自己的 WebView 解開也一樣沒用。

**結論不變，但原因更精確**：`src/fetch.rs` 已經拿掉所有 `imports::js::WebView`
相關程式碼（連 8.6 版保留給 `fetch_html` 的 fallback 也一併拿掉，理由跟 `fetch_json`
完全相同，不分網域），只保留 direct `Request::send()` + `cf-mitigated` header／body
字串偵測失敗訊息。**不會再嘗試自己開 WebView 解 Cloudflare**——這個方向已經被
host app 原始碼證實結構上做不到，不是我們的實作問題。

**唯一還沒驗證、值得使用者實機確認的一點**：`CloudflareHandler.completeChallenge`
需要 `UIApplication.shared.appDelegate?.visibleViewController` 才能掛上隱形/彈出
webview（`addWebView` 拿不到就直接 `throw .missingParentView`，整套機制連跑都不會跑，
安靜地把原始驗證頁丟回來，使用者完全不會看到任何彈窗——這跟目前的回報症狀吻合）。
如果章節更新是透過 App 背景重新整理書庫（而不是使用者正在前景主動點開某本漫畫）觸發
的，這個時間點很可能沒有合適的 view controller 可以掛，導致彈窗機制整套跳過。**值得
請使用者測試比較**：(a) 背景庫更新時失敗 vs (b) 前景主動點進去同一本漫畫、同一個章節
時是否也失敗——如果 (a) 失敗但 (b) 成功，代表問題出在背景執行沒有畫面可掛彈窗，而不是
挑戰本身真的解不開；如果兩者都失敗，才真的比較接近「這個 zone 對我們的流量已經被升到
真人必點」的結論。

### 8.9 使用者確認：前景主動點進去一樣失敗，parentView 假設被排除（2026-09-21）

使用者實測：前景主動點開該漫畫/該章節，用的是新版（拿掉 WebView、`cf-mitigated`
header 偵測）的 build，log：

```
[zh.hip] Error: Message("hit CF challenge (cf-mitigated header, status=403)")
```

排除了 8.8 最後提出的「背景更新沒有 parentView 可掛彈窗」假設——前景操作照樣失敗，
代表 `CloudflareHandler` 的整套機制（含彈窗）**真的有機會跑，但還是沒解開**。

重新看了一次 `CloudflareHandler.swift` 的 `isCaptchaPage()`，發現一個**可能**（未在
真機上驗證，只是靜態讀程式碼推論）的 JS 運算子優先序問題：

```javascript
(document.querySelector('input[name="cf-turnstile-response"]') !== null
    || document.getElementById('challenge-error-title') !== null
    || document.getElementById('challenge-error-text') !== null) ? 1 : 0
    || document.title === "Just a moment..."
```

JS 的 `||` 優先序比三元運算子 `?:` 高，所以這段實際上會被解析成
`(A||B||C) ? 1 : (0 || (document.title === "Just a moment..."))`。當前三個條件都不成立、
但 `document.title` 確實是 `"Just a moment..."` 時，整個運算式的值會是**布林
`true`**，不是整數 `1`。Swift 端 `guard let result = result as? Int else { return
false }` 如果拿到的是 JS 的 boolean `true`（而不是 number），`as? Int` 轉型**可能**失敗
（取決於 `WKWebView.evaluateJavaScript` 把 JS boolean 橋接回 Swift 時的實際型別），一旦
失敗就直接 `return false`——導致明明卡在「Just a moment...」驗證頁，`isCaptchaPage()`
卻回報「不是驗證頁」，於是 12 秒逾時前都不會觸發 `showPopup`，使用者永遠看不到彈窗，
整個挑戰安靜逾時失敗。

**這只是一個有理論依據、但沒辦法在這個 repo 裡驗證的猜測**（沒有 iOS 裝置/Xcode 可以
實測 JS boolean 橋接到 Swift `Int` 的實際行為），如果屬實，這是 host app
（`github.com/Aidoku/Aidoku`）本身的 bug，不是 `zh.hip` 這邊能修的，值得使用者評估要不要
去 Aidoku 那邊回報，附上這段程式碼位置（`CloudflareHandler.swift` 的
`isCaptchaPage()`）。

**對 `zh.hip` 這個 repo 的結論**：查證已經到頭——`aidoku-rs`（Rust SDK）、
`Aidoku`（host app）、`AidokuRunner`（wasm 執行環境）、`Aidoku-Community/sources`
（社群先例）四個地方都查過，沒有任何 source 端能做的事。目前 `fetch.rs` 的寫法
（direct-only + `cf-mitigated` header 快速失敗）已經是這個問題在現有 SDK/app 能力範圍
內最好的處理方式，之後除非 Aidoku app 端自己修掉 `isCaptchaPage()` 這類問題，否則不用
再回來調這段程式碼。

### 8.10 使用者確認：彈窗真的有跳、也真的點了，但點完照樣再要求驗證（2026-09-21）

使用者補充關鍵細節：**「有彈出勾選驗證但點了之後還是會再要求驗證」**。

這推翻了 8.9 的 `isCaptchaPage()` 假設——彈窗確實有跳出來，代表 8.9 那個 JS 運算子
優先序猜測不是這次卡住的原因（`isCaptchaPage()` 有正確判斷出是驗證頁並觸發
`showPopup`）。問題出在**更後面一步**：使用者已經手動完成驗證，但緊接著的重試請求
還是被擋。

對照 `CloudflareHandler.handle()` 的邏輯：驗證完成後只會**重打一次**原始請求
（`URLSession.shared.data(for: newRequest)`），如果那一次重試仍然被判定為挑戰頁
（`shouldHandle` 再次為真），就直接 `throw .solveFailed`，把原始被擋的回應原封不動丟
回 source——不會再跳第二次彈窗、不會重試第二次。也就是說**每次呼叫 `handle()` 只給
使用者一次機會**，這次失敗了就結束，換下一次不同的請求觸發下一輪全新的
`awaitChallenge` → 彈窗 → solve → retry-once 循環，體感上就是「點了驗證還是一直要求
驗證」。

**為什麼手動點過驗證後，緊接著的重試還是會被擋？** 有兩個可能，都不是我們能從
`zh.hip` 這邊修的：

1. **Cookie 橋接的時間差**：`navigated()` 偵測到新 `cf_clearance` 後才呼叫
   `finishChallenge()` 恢復 `handle()` 的 continuation，理論上此時 cookie 已經寫進
   `HTTPCookieStorage.shared`，但如果偵測/寫入之間有極小的時間差，緊接著發出的重試
   請求有機會搶在 cookie 真正生效前發出。
2. **更可能：`cf_clearance` 在 Cloudflare 那端可能綁定的不只是 cookie 值，還有發出
   請求的網路/TLS 指紋**。`WKWebView` 解開驗證用的是它自己的網路堆疊，緊接著的重試
   請求改用 `URLSession.shared`——iOS 上這是兩條不同的網路堆疊，TLS handshake 的
   指紋（JA3/JA4）不一定相同。如果 Cloudflare 對這個 zone 的 bot management 設定
   除了驗證 cookie 之外還會比對請求指紋的一致性，那即使 cookie 值正確帶過去，
   `URLSession` 端的重試依然可能被判定為不同的（未受信任的）客戶端，重新打回挑戰頁。
   這跟 `zh.happy` 研究記錄裡「reqwest 的 TLS 指紋被 Cloudflare 擋掉」是同一類問題，
   只是這次發生在 host app 自己內建、理論上最完整的機制上，不是我們自己拼湊的
   WebView。

**最終結論（這個判定現在建立在比 8.6 更扎實的證據上）**：連 host app 自己最完整的
自動化＋真人手動驗證機制，實測都無法讓 `hipapi1.s3file.top` 穩定放行——不是我們的
WebView fallback 不夠好，是 app 內建最好的那一套在使用者親自點過驗證的情況下依然
失敗。這跟 `zh.baka`（memory `project-baka-cloudflare-blocked`）「決定性證實無解」是
同一個等級的結論，只是根因細節（推測是 cookie 與網路指紋綁定不一致，而非純粹的
「互動式 vs 自動」挑戰分級）更明確一點。`zh.hip` 這個 repo 沒有更多能做的事；如果要
繼續往下查，下一步是去 `github.com/Aidoku/Aidoku` 回報，帶上這裡整理的證據鏈
（`CloudflareHandler.swift` 解出挑戰後只 retry-once、`WKWebView`/`URLSession` 網路
堆疊不同這兩點），但那已經不是 `zh.hip` 原始碼能解決的範圍。

### 8.11 唯一還沒試過、成本可接受的 source 端手段：多打一次全新的請求（2026-09-21）

使用者問「能不能從 source 這邊克服」。重讀 `CloudflareHandler.handle()` 後發現一個
還沒試過、邏輯上站得住腳的做法：**它在使用者解完驗證後只重打一次原始請求，那一次
失敗就直接放棄，不會再給第二次機會**；但如果我們自己的 Rust 程式碼在偵測到驗證頁後
再發一個**全新的** `Request::send()`，這對 `CloudflareHandler` 來說是一個全新的請求，
會從頭跑一次完整流程（含視需要再跳一次彈窗），不是去重用剛剛那次已經判定失敗的結果。

這**不保證有用**——如果 8.10 猜的「cookie 跟連線/TLS 指紋綁定」是結構性、每次都
100% 重現的問題，重試一樣會 100% 失敗；但如果失敗原因帶有機率成分（例如 cookie 橋接
的極短時間差、或 Cloudflare 當下風險分數評得剛好比較嚴格），多一次獨立的完整流程就
有機會撐過去。成本可接受：失敗才會觸發，多等一輪、使用者可能要再點一次驗證，不會更糟。

**修法**（`src/fetch.rs::fetch_text`）：包成最多 2 次嘗試的迴圈（`MAX_ATTEMPTS = 2`），
第一次偵測到驗證頁就重打一次全新請求，兩次都失敗才真的回傳錯誤。故意只設 2 次、不是
更多次——避免使用者被連續彈好幾次視窗。build/clippy 乾淨。

**還沒驗證**：這純粹是根據 `CloudflareHandler.handle()` 程式碼邏輯推出來的合理嘗試，
沒辦法在這個 repo 裡確認到底有沒有幫助，需要使用者實機測試。如果兩次都還是一樣的
`cf-mitigated` 錯誤（尤其是彈窗兩次都跳、兩次都點了還是失敗），那就是 8.10 的
「結構性、非機率性」結論被進一步坐實，之後不用再期待重試次數能解決，源頭問題在
Cloudflare／host app 那一側，不在 `zh.hip`。

### 8.12 使用者提問：改 User-Agent 有沒有用？重讀 CloudflareHandler 找到具體理由（2026-09-22）

重讀 `CloudflareHandler.addWebView(for:)`（8.8 節查過的同一個檔案）發現一個先前沒特別
注意的細節：

```swift
let userAgent = request.value(forHTTPHeaderField: "User-Agent")
let config = WKWebViewConfiguration()
if let userAgent, userAgent.contains("iPhone") || userAgent.contains("iPad") {
    config.defaultWebpagePreferences.preferredContentMode = .mobile
}
webView = WKWebView(frame: .zero, configuration: config)
webView.customUserAgent = userAgent
```

**app 用來解 Cloudflare 驗證的那個 WebView，會直接套用觸發挑戰的那個請求本身帶的
User-Agent。** `zh.hip` 的 `settings.rs::get_user_agent()`（改之前）預設是硬編碼的
**桌機版 Chrome 135** 字串，而且不是使用者自己填的，是我們的程式碼主動塞的。如果
實際裝置是手機，代表解驗證的 WebView 對 Cloudflare 宣稱自己是桌機 Chrome，但背後的
連線/TLS 指紋來自一支手機——這正是 bot 偵測常見的訊號不一致，也可能是 8.10 猜測的
「trust 綁定連線指紋」問題的**一部分成因**（不一定是全部，因為 8.10 的彈窗測試發生在
改這個之前）。

比對 `zh.happy`／`zh.baka` 兩個手足專案的 `fetch.rs`，兩者都**完全沒有**自訂
User-Agent，直接讓 `AidokuRunner.swift` 的 `Source.modify()`
（`if request.value(forHTTPHeaderField: "User-Agent") == nil { ... UserAgentProvider ... }`）
自動填入跟裝置一致的值。`zh.hip` 這個桌機 UA 偽裝，對照 RESEARCH.md 開頭「從
`zh.mxs` 複製貼上開始」的背景，比較像是繼承來的樣板，沒有證據顯示是針對這個站驗證過
必要的設定。

**修法**：

- `res/settings.json`：`userAgent` 欄位的 `"default"` 改成空字串（保留
  `"placeholder"` 提示「留空則使用 app 預設」），不再讓使用者一開始就看到/帶著一個
  桌機 UA。
- `settings.rs::get_user_agent()`：回傳型別改成 `Option<String>`，使用者沒填就回
  `None`，拿掉原本「空值時自動寫入硬編碼預設值」的副作用（那個寫入本身也是個問題：
  等於把偽裝值永久寫進使用者的設定儲存區，之後即使我們改程式碼，已安裝過舊版的
  使用者裝置上仍然殘留舊的硬編碼值，直到使用者自己動手清掉那個設定欄位）。
- `fetch.rs::Fetch::request()`：只有 `get_user_agent()` 回傳 `Some` 時才附加
  `User-Agent` header；`None` 時完全不附加，讓 `Source.modify()` 自動填入跟裝置
  身分一致的值。

保留設定頁的自訂欄位本身（沒有整個拿掉），使用者如果真的需要手動指定 UA（例如某些
站對特定 UA 有不同行為）還是可以自己填。build/clippy 乾淨。

**還沒驗證**：同樣沒辦法在這個 repo 裡確認有沒有幫助，需要使用者實機測試。如果換掉
偽裝 UA 之後 8.11 的重試機制開始偶爾成功，代表 UA 不一致確實是（部分）根因；如果依然
100% 失敗，代表 8.10 猜的 TLS/連線指紋問題比 UA header 本身更底層，UA 只是次要因素，
真正的根因還是不在 source 端能碰到的範圍。

### 8.13 兩次嘗試都失敗，結構性結論確定（2026-09-22）

使用者實機測試（已套用 8.12 的 UA 修法）結果：

```
[zh.hip] Error: Message("hit CF challenge (cf-mitigated header, status=403, attempt=2)")
```

`attempt=2` 代表 8.11 的兩次獨立完整流程（每次都是全新的 `Request::send()`，都會讓
`CloudflareHandler` 從頭跑一次，包含視需要再跳一次彈窗）**都失敗**，不是碰巧兩次都倒楣
——換掉桌機 UA 偽裝、給機率成分一個機會之後，結果沒有任何改善。

**結論不再是「猜測」，是兩輪獨立驗證都指向同一個方向**：

1. 8.10：真人手動點過彈窗驗證，下一個請求還是被擋。
2. 8.13（本節）：拿掉 UA 不一致這個變因、多給一次完整重試機會，結果依然 100% 失敗。

`hipapi1.s3file.top` 對我們這個來源的流量套用的 Cloudflare 保護，已經確定**不是**
UA 訊號不一致、也**不是**單純運氣/時機問題能解釋的——比較符合 8.10 猜測的「trust
綁定在比 cookie 更底層的訊號（例如連線層級的指紋）」，這種層級的問題不管是
`imports::js::WebView`（8.8 已證實結構上不可能有用）、UA header、或多打幾次請求，
都在 source 程式碼能碰到的範圍之外。

**這個 repo 到此為止，不會再嘗試新的程式碼修法**。已經依序查證過：
`aidoku-rs`（Rust SDK，無相關 API）、`Aidoku`（host app，`CloudflareHandler` 含真人
彈窗，實機確認仍失敗）、`AidokuRunner`（wasm 執行環境，source 的 WebView 用隔離
cookie store）、`Aidoku-Community/sources`（社群先例，沒人解過同類問題），外加
UA、重試次數兩個實機驗證過無效的緩解。跟 `zh.baka` 是同一個等級的結論：目前
`fetch.rs` 的寫法（direct + 一次重試 + 清楚快速的錯誤訊息）已經是能做到的最佳處理。
如果之後還要繼續，下一步只剩去 `github.com/Aidoku/Aidoku` 回報，不是改 `zh.hip`
的程式碼。

### 8.14 決定性反證：一般桌機瀏覽器（無 Aidoku、無 WebView、無 TLS 指紋問題）也一樣卡驗證迴圈（2026-09-22）

使用者直接用電腦瀏覽器開 `https://m.hipmh.com/works/bToyMzQ3NQ-yi-ren-zhi-xia-tx-531490-17793`
（本節這個書籍詳情頁，不是 `hipapi1.s3file.top` 的 JSON API）——**一開就卡在驗證畫面，
點了驗證還是一直循環跳回來**。

這是目前為止最重要的一筆證據，因為它**排除了 8.10/8.13 建立起來的整條「Aidoku app
架構特有」推論鏈**：一台普通桌機瀏覽器不會有 `WKWebView` vs `URLSession` 兩條網路
堆疊、不會有 iOS 裝置的連線/TLS 指紋、不會有我們自己 spoof 的 User-Agent、也不會有
`imports::js::WebView` 的隔離 cookie store 問題——這些全部是 Aidoku 這邊才有的
概念，跟一般桌機 Chrome/Safari 完全無關。**如果連這種最乾淨的環境都卡在同一個
「點了驗證還是一直循環」的迴圈裡，代表問題根本不是 Aidoku app 的網路架構造成的，
而是站方自己這個 Cloudflare zone（至少對某些沒被 CDN 快取住的頁面/路徑）的驗證
設定本身就有問題（例如 Managed Challenge/Turnstile widget 設定錯誤、或該 zone 的
挑戰驗證流程卡在無限迴圈），對任何客戶端都一樣——不分裝置、不分是不是 app、
不分 TLS 指紋。**

值得注意：同一顆網址我自己剛才用 curl 測是 `200 OK`（`cf-cache-status: HIT`，
`Age: 59440` 秒 ≈ 16.5 小時前的快取），完全沒有卡驗證。這跟使用者「一開就卡住」
的經驗不矛盾，反而互相印證了整份文件从 8.2 開始就一直觀察到的規律：**已經被
CDN 快取住的請求會直接放行、繞過驗證；沒被快取住、真的打到 origin 的請求才會
觸發驗證**，只是這次多了一個新事實——**觸發驗證之後，這個 zone 的驗證流程對
任何客戶端（不只 Aidoku）看起來都會卡在無限迴圈，不會真的放行**。

**8.10（app 彈窗解完還是被擋）、8.13（UA 修正 + 重試兩次仍 100% 失敗）這兩節的
「可能是連線/TLS 指紋綁定」推論，現在看來多半是想太多了**——不需要那麼複雜的
解釋，直接原因很可能就是**這個網站的 Cloudflare 驗證流程本身壞了**，任何人（含
拿真人瀏覽器手動點驗證）打到未快取的頁面都會卡住，Aidoku／iOS／`zh.hip` 完全是
無辜的，UA header、重試次數、WebView 這些先前排查的方向從一開始就不會是解法，
因為問題根本不在客戶端這一側。之前記錄裡「可能是 TLS 指紋不一致」的猜測予以
保留但降低權重，改標記為「不是必要解釋，更簡單的『站方驗證流程本身壞掉』已經
能完整解釋所有觀察到的現象」。

**結論不變，但原因更正**：不管是哪一種解釋，都一樣**不是 `zh.hip` 這個 repo 能修的
問題**——如果是站方 Cloudflare 設定壞掉，只有站方自己能修（可能哪天他們自己修好
就正常了）；如果使用者想繼續確認，可以直接拿瀏覽器去該站回報，或是過一段時間
再重測這個網址看驗證迴圈是否還在，這樣比繼續調 `zh.hip` 的程式碼更有機會看到
變化。

### 8.15 使用者發現的 `/cdn-cgi/challenge-platform/.../jsd/oneshot/...` 網址（2026-09-22）

使用者從瀏覽器（推測是開發者工具 Network 面板）撈到這個網址：

```
https://m.hipmh.com/cdn-cgi/challenge-platform/h/g/jsd/oneshot/330e41bb475c/
  0.28014231027059655:1790003414:n3aqYV4sIrq31WtzOnShmTMvYD-jQPtTyZpE5etmNIw/
  a3ea52ca089f15eb
```

拆解：`/cdn-cgi/challenge-platform/` 是 Cloudflare 自己掛在**每一個受保護網域**下的
固定路徑（不是站方寫的程式，是 Cloudflare 邊緣節點自動插入的），`jsd` = **JS
Detection**，`oneshot` 代表這個腳本實例是單次性的（每次頁面載入產生一個新的，帶
時間戳記+隨機數+簽章的 token，不能重複使用）。這是 Cloudflare **進階 Bot
Management** 產品的一部分：瀏覽器背景載入這支腳本，持續收集裝置/瀏覽器環境的
訊號（不只是使用者眼睛看得到的驗證畫面那一次），把結果回報回這個端點，餵給
Cloudflare 對這個訪客的風險分數。用 curl 直接打這個網址回 `405 Method Not
Allowed`、`allow: POST`（`Server: cloudflare`）——確認這是 Cloudflare 自己的回報端點，
只接受瀏覽器 JS 用 POST 主動回報，不是拿來 GET 資料用的，跟這一個 token 綁定的那次
頁面載入脫鉤後單獨打它本來就不會有意義的回應。

**這跟目前的驗證迴圈有沒有關係**：很可能有，而且提供了一個比 8.10/8.13 猜的「連線/
TLS 指紋」更具體、更貼近 Cloudflare 實際產品設計的解釋——**如果這個背景 JS 偵測
腳本沒有正常執行完、或它回報的訊號一直被判定為可疑**（例如被瀏覽器隱私設定/
擴充套件擋掉、網路環境異常、或單純 Cloudflare 這個 zone 的風險評分邏輯本身有 bug），
就可能導致即使使用者手動點過眼睛看得到的驗證畫面，Cloudflare 背後的風險分數依然
沒有被這支持續回報的腳本「洗白」，於是還是繼續判定這個訪客可疑、繼續要求驗證，
形成使用者看到的無限迴圈。這完全屬於 Cloudflare 自己的 Bot Management 邊緣層
運作機制，跟站方的程式碼、Aidoku app、更不用說 `zh.hip` 的原始碼都沒有關係——
呼應 8.14「這是站方 Cloudflare zone 本身的問題」的結論，只是這次多了一個具體的
技術環節（JS Detection 持續回報這一步可能卡住）可以指認。

### 8.16 已恢復（2026-09-22）

使用者回報 `https://m.hipmh.com/popularity` 恢復正常。實測確認**不是只有快取在撐**——
特地重測幾個先前 100% 會觸發驗證的路徑：

- `hipapi1.s3file.top/`（根路徑，本來就不是會被有意義快取的端點，之前每次都直接
  回驗證頁）→ 現在 `200`，回傳 `{"description":"High-performance manga platform
  API",...}`
- `v1/manga/chapters?...&per_page=3&order=asc`（8.2 最早重現問題的那個冷門組合）
  → 現在 `200`
- `v2/chapter?hid=...`（章節圖片 API，使用者最原始回報的那個端點，換一個從沒打過
  的 hid）→ 現在 `200`
- `m.hipmh.com/works/bToyMzQ3NQ-...`（使用者瀏覽器卡住的那個書籍詳情頁）→ 現在
  `200`，內容正常

以上全部是先前**確定會觸發驗證、不是靠快取繞過**的請求，現在全部放行。這證實
8.14/8.15 的判斷方向：問題出在站方 Cloudflare zone 當時的風險評分/驗證流程被卡住
或升級，屬於暫時性狀態，不是永久封鎖設定，現在已經自己恢復。

**對 `zh.hip` 程式碼的影響**：沒有需要改的地方。目前的實作（direct request + 一次
重試 + `cf-mitigated` header 快速偵測 + 不偽裝 UA）在協定層面已經是正確的寫法，
先前測試失敗完全是外部站方狀態的問題，不是程式碼邏輯錯誤，現在站方恢復後應該會
恢復正常運作。**保留這整節 8.1–8.16 的診斷記錄**：如果之後又復發同樣的症狀（換章節
失敗、驗證迴圈），這是已知會發生、有完整根因分析的已知現象，不用重新排查一次，
直接對照這裡的結論；如果復發時規律不同（例如真的是每次都卡、從不恢復），才需要
重新記錄新的差異。

### 8.17 補充修正：電腦與手機共用家用 WiFi，更支持「單純 IP／行為風險分數」而非站方設定壞掉（2026-09-22）

> **⚠️ 本節結論已於 2026-09-25 被否證，見第九節。** 下面推論的「同一個 IP 的風險分數被
> 手機使用 + 密集測試的疊加流量推高」無法解釋後續觀察到的現象：**同一台 router 上的電腦
> 瀏覽器可以連續正常操作，而手機 Aidoku 同時段打同一顆主機必定被擋**。IP 相同、請求量還
> 是瀏覽器比較多，結果卻相反——判別變數不在網路層。本節保留作為推論過程記錄，但**不要
> 再據此推導任何「減少請求量」型的解法**（那正是 9.2 記錄的一次失誤）。

使用者補充：8.14 節「電腦瀏覽器也卡住」跟手機（跑 `zh.hip`）走的是**同一個家用
WiFi**，也就是**同一個對外 IP**。這修正了 8.14 當時的推論——原本以為電腦跟手機是
兩個獨立網路環境，兩邊都卡住比較像是「站方全域壞掉」；現在知道兩邊其實是同一個
IP，這個現象改用「單一 IP 的風險分數被拉高」就能完整解釋，不需要假設站方
Cloudflare 設定本身有 bug：

- 手機端 `zh.hip` 反覆切換章節、加上這次除錯過程新加的 2 次重試（8.11），對同一個
  IP 持續產生看起來像自動化的流量。
- 我自己在這台機器上密集 curl 測試（短時間 40-50+ 次請求），如果剛好也落在鄰近的
  時間窗口，兩邊疊加會讓風險分數更難降下來。
- 8.10「使用者在電腦上手動點過驗證，下一刻又被要求驗證」這個最像「無限迴圈」的
  症狀，如果當下手機那邊還在跑（同一個 IP 持續有其他可疑請求進來），也可以合理
  解釋成「這邊剛解開、那邊又把分數推高」，不必然是 Cloudflare 平台本身的 bug 或
  `cf_clearance` 沒生效。
- 現在（隔了一段時間，雙方都沒有再密集測試）風險分數自然降下來，所有先前卡住的
  端點才會一起恢復——時間點跟「單一 IP 暫時性限制、過一段時間自動解除」的模式
  完全吻合。

**修正結論**：8.14 節「站方 Cloudflare 設定本身壞掉」的判斷下修為**次要可能性**，
主要解釋改成**這次的驗證迴圈很可能就是單純的 IP／行為風險分數機制在正常運作**，
只是被我們自己（手機使用 + 密集測試除錯）的組合流量觸發，不代表站方有問題、也
不代表 Aidoku／`zh.hip` 有程式邏輯錯誤。8.15 節查到的 JS Detection 背景評分機制
（Cloudflare Bot Management 用來持續評估訪客風險）在這個修正後的框架下更說得通：
它評的本來就是連線/裝置/行為訊號的綜合風險，不是單純比對 cookie 有沒有帶對。

對 `zh.hip` 本身沒有新的行動項——不管是「站方 bug」還是「IP 風險分數」，都一樣不是
程式碼能控制的外部因素，8.16 的「現狀已恢復、程式碼不用改」結論不變。唯一實務上
值得記住的教訓：**之後除錯這類 CF 問題時，盡量避免「使用者在同一個網路上一邊密集
測試 app、我這邊又同時密集用 curl 打同一個站」——兩邊疊加很容易把原本可能只是
偶發的驗證，推成看起來像「完全解不開」的假象**，拖慢真正的診斷。

### 8.18 User-Agent 改動已還原（2026-09-22）

8.12 的 User-Agent 修法（拿掉桌機版 Chrome 偽裝，改讓 app 自動填一個跟裝置一致的
值）最後被使用者要求還原回原本硬編碼桌機 UA 的寫法。理由：

- 8.13 的實測結果是**套用 UA 修法之後**測的，兩次嘗試依然 100% 失敗——這個修法
  沒有觀察到任何幫助。
- 額外實測確認：這個站（`m.` 開頭本身就是站方自己的行動版網域）**不管 UA 是桌機
  還是手機，回傳的頁面內容完全一致**（逐 byte 比對 93130 bytes 一模一樣），所以
  UA 選擇對內容解析／選擇器完全沒有影響，換不換都不影響 `html.rs` 的解析邏輯。
- 8.17 修正後的主因判斷（單純 IP／行為風險分數，被密集測試流量觸發）跟 UA header
  本身關係不大，UA 訊號不一致的理論已經是次要假設，不足以構成保留這個改動的理由。

`src/settings.rs::get_user_agent()`、`res/settings.json` 的 `userAgent` 欄位、
`src/fetch.rs::Fetch::request()` 都已還原成本節之前的原始寫法（硬編碼桌機版
Chrome 135 當預設值，使用者沒填設定就自動寫入這個預設並附加在每個請求上）。
build/clippy 乾淨。`fetch.rs` 其餘部分（`cf-mitigated` header 偵測、拿掉 WebView
fallback、2 次重試）維持不變，那些跟這次的 UA 決定是各自獨立的修法，理由沒有
被推翻。

**注意：8.18 完成後，使用者要求把 `zh.hip` 程式碼整個 `git checkout` 回這整份
8.1–8.18 診斷開始之前的原始狀態**（只保留 `docs/RESEARCH.md` 本身）。也就是說
截至本節為止，`src/fetch.rs`、`src/html.rs`、`src/json.rs`、`src/lib.rs`、
`res/source.json` 目前都是**未套用任何本文件修法的原始版本**——`fetch_json`／
`fetch_html`、`cf-mitigated` 偵測、拿掉 WebView fallback、2 次重試全部都不存在，
`html.rs`/`json.rs` 用的還是最原始的 `Fetch::get(url)?.json_owned()?`／`.html()?`。
之後如果要重新套用這裡任何一節的修法，程式碼要重寫，不能假設它已經在檔案裡。

### 8.19 復發記錄（2026-09-22，程式碼已還原後）

使用者回報 `https://hipapi1.s3file.top/v1/manga/chapters?mid=bToyMzQ3NQ&page=1&per_page=20&order=desc`
又壞了。curl 驗證：

```
$ curl -D - "https://hipapi1.s3file.top/v1/manga/chapters?mid=bToyMzQ3NQ&page=1&per_page=20&order=desc"
HTTP/1.1 403 Forbidden
Date: Mon, 21 Sep 2026 16:26:19 GMT   # 系統時鐘顯示 Mon 但當下實際日期是 2026-09-22
Content-Length: 5890
Cf-Mitigated: challenge
Server: cloudflare
Server-Timing: chlray;desc="a3ea6e507ffccf22"
<!DOCTYPE html><html lang="en-US"><head><title>Just a moment...</title>...
```

追加確認**不是只有這個冷門的 `per_page=20` 組合被擋**——連先前一直穩定命中快取、
被視為「安全」的組合也一起被擋了：

- `per_page=50&order=desc`（`zh.hip` 原始程式碼實際使用的組合，8.16 確認過的
  「恢復」對象）→ 同樣 `403` + `Just a moment`
- `hipapi1.s3file.top/` 根路徑（不可快取的 API info 端點，8.16 也確認過的
  「恢復」對象）→ 同樣 `403` + `Just a moment`

也就是說這次復發**是整個網域對這次測試流量的全域擋下**，跟 8.16 恢復之前觀察到的
症狀是同一種模式（不是單一 URL 的問題），符合 8.17「IP／行為風險分數被拉高」的
判斷——而且這次很可能又是這個對話 session 自己造成的：緊接著使用者回報之前，
這裡才剛連續發了好幾次 curl 請求做前面的驗證（8.16 的復原確認、8.18 前後的比對
測試等），疊加使用者自己的操作，很可能又把同一個 IP 的風險分數推回去了——直接
重演 8.17 記錄的教訓。

**使用者指示**：先把這次復發的必要資訊記錄下來（本節），之後等它再度恢復時，
拿現在的紀錄跟屆時的狀態比對，而不是立刻動手改程式碼。目前**不對程式碼做任何
變更**，純粹觀察等待。

**完整記錄**（`per_page=20&order=desc` 這次觸發的請求，2026-09-22 16:28:46 GMT）：

完整 response header：

```
HTTP/1.1 403 Forbidden
Date: Mon, 21 Sep 2026 16:28:46 GMT
Content-Type: text/html; charset=UTF-8
Content-Length: 5890
Connection: close
Accept-Ch: Sec-CH-UA-Bitness, Sec-CH-UA-Arch, Sec-CH-UA-Full-Version, Sec-CH-UA-Mobile, Sec-CH-UA-Model, Sec-CH-UA-Platform-Version, Sec-CH-UA-Full-Version-List, Sec-CH-UA-Platform, Sec-CH-UA, UA-Bitness, UA-Arch, UA-Full-Version, UA-Mobile, UA-Model, UA-Platform-Version, UA-Platform, UA
Cf-Mitigated: challenge
Content-Security-Policy: default-src 'none'; script-src 'nonce-m1GOJ7ezCApvMEoILquClQ' 'unsafe-eval' https://challenges.cloudflare.com; script-src-attr 'none'; style-src 'unsafe-inline'; img-src 'self' https://challenges.cloudflare.com; connect-src 'self' https://challenges.cloudflare.com; frame-src 'self' https://challenges.cloudflare.com blob:; child-src 'self' https://challenges.cloudflare.com blob:; worker-src blob:; form-action http: https:; base-uri 'self'
Server: cloudflare
Critical-Ch: Sec-CH-UA-Bitness, Sec-CH-UA-Arch, Sec-CH-UA-Full-Version, Sec-CH-UA-Mobile, Sec-CH-UA-Model, Sec-CH-UA-Platform-Version, Sec-CH-UA-Full-Version-List, Sec-CH-UA-Platform, Sec-CH-UA, UA-Bitness, UA-Arch, UA-Full-Version, UA-Mobile, UA-Model, UA-Platform-Version, UA-Platform, UA
Cross-Origin-Embedder-Policy: require-corp
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Resource-Policy: same-origin
Origin-Agent-Cluster: ?1
Permissions-Policy: accelerometer=(),camera=(),clipboard-read=(),clipboard-write=(),geolocation=(),gyroscope=(),hid=(),magnetometer=(),microphone=(),payment=(),publickey-credentials-get=(),screen-wake-lock=(),serial=(),sync-xhr=(),usb=(),xr-spatial-tracking=*
Referrer-Policy: same-origin
Server-Timing: chlray;desc="a3ea71e54d7326d3"
X-Content-Type-Options: nosniff
X-Frame-Options: SAMEORIGIN
Report-To: {"group":"cf-nel","max_age":604800,"endpoints":[{"url":"https://a.nel.cloudflare.com/report/v4?s=..."}]}
Nel: {"report_to":"cf-nel","success_fraction":0.0,"max_age":604800}
CF-RAY: a3ea71e54d7326d3-SJC
alt-svc: h3=":443"; ma=86400
```

跟 8.2 最早重現問題時的 header 組合比對，**這次多了一批先前沒觀察到的安全性
header**（`Accept-Ch`／`Critical-Ch`／`Cross-Origin-Embedder-Policy`／
`Cross-Origin-Opener-Policy`／`Cross-Origin-Resource-Policy`／
`Origin-Agent-Cluster`／`Permissions-Policy`／`Referrer-Policy`／
`X-Content-Type-Options`／`X-Frame-Options`），這些是 Cloudflare Client Hints／
安全性 header 政策的一部分，可能代表 Cloudflare 在這期間更新過這個 zone 的邊緣
設定（或本來就有、只是先前擷取時剛好省略/截斷沒注意到），單純記錄下來，暫不
下結論。

body 裡的挑戰設定（`window._cf_chl_opt`）解出來的關鍵欄位：

```
cType: 'managed'      # 跟 8.2 觀察到的一致，還是 managed challenge，不是 interactive
cZone: 'hipapi1.s3file.top'
cRay: 'a3ea71e54d7326d3'   # 跟 CF-RAY header 一致
cvId: '3'
```

`cH`／`md`／`mdrd`／`cUPMDTk`／`fa` 等欄位是單次性、跟這個 ray id 綁定的 token，
換一次請求就會變，沒有記錄價值（重打會拿到全新的值），故略過。

**比對：這次恢復只花了約 5 分鐘（2026-09-22 16:33:57 GMT）**：

```
HTTP/1.1 200 OK
Date: Mon, 21 Sep 2026 16:33:57 GMT
cf-cache-status: MISS
CF-RAY: a3ea797c6f93ed3b-SJC
```

`cf-cache-status: MISS` 是這次比對裡最關鍵的一點：代表這是**真的打到 origin 拿到
的全新回應**，不是被 CDN 快取矇混過去（8.16 那次恢復確認用的是快取 HIT 的頁面，
沒有直接證明 origin 本身已經放行；這次是 MISS，直接證明 origin／挑戰路徑真的
恢復了）。

擋下時間 `16:28:46` 到恢復時間 `16:33:57`，**中間只隔了約 5 分鐘**，比 8.16 那次
「隔了將近一天才恢復」快非常多。這個時間差本身是新資訊：代表這次的風險分數/
限制視窗遠比第一次短——跟 8.17「風險分數會隨流量停止而自然冷卻」的推論吻合，
且進一步支持「不是站方設定壞掉、而是短時間流量觸發的動態限制」這個判斷，因為
真正的設定錯誤/bug 不會這樣忽快忽慢地自己時好時壞。

**第三個獨立網路路徑交叉驗證**：除了這裡的測試環境跟使用者家用網路之外，另外用
完全不相關的第三方網路（Claude 的網頁擷取服務）抓同一顆書籍詳情頁，內容完全正常
（標題「一人之下」、作者「米二」「米橙子」、813 章、簡介都正常顯示，沒有任何
Cloudflare 驗證頁特徵）。三個互不相關的網路路徑（測試環境 IP、使用者家用 IP、
第三方擷取服務）同時都正常，排除「只是我們這兩個一直在測試的 IP 剛好被放行、
其他訪客可能還在卡」的可能性，進一步支持這是**整個網域對外層級的恢復**，不是
針對特定來源的局部解除。

**能比對到的「為什麼」就到此為止**：Cloudflare 實際的風險評分邏輯、規則設定是
平台內部黑盒，站方自己登入 Cloudflare 後台才看得到，純粹從外部發請求觀察
（header、body 裡的 `_cf_chl_opt`、CF-RAY、時間差）已經是我們能取得的全部資訊，
沒有更進一步的比對方式可以直接看到「內部到底發生了什麼變化」。

### 8.20 再次確認已恢復（2026-09-25）

三天後重測 8.19 記錄的三個端點，全部 `200`：

```
v1/manga/chapters?mid=bToyMzQ3NQ&page=1&per_page=20&order=desc  → 200, cf-cache-status: MISS
hipapi1.s3file.top/（根路徑）                                     → 200, cf-cache-status: BYPASS
m.hipmh.com/works/bToyMzQ3NQ-...（書籍詳情頁）                    → 200, cf-cache-status: HIT
```

前兩個是 `MISS`/`BYPASS`，代表是真的打到 origin 拿到的放行回應，不是快取矇混過去；
跟 8.19 描述的「短時間流量觸發的動態限制、會自己冷卻」模式一致。目前沒有新現象、
沒有新資訊要記錄，純粹是例行比對確認狀態穩定。程式碼依照 8.18 的使用者指示維持
還原狀態，不套用任何修法；如果之後再復發，比照本節格式記錄時間點與端點狀態即可。

### 8.21 問題範圍其實比想像窄：圖片 CDN 不在 Cloudflare 後面（2026-09-25）

追查「referer 有沒有用」跟 reader 網域的 `jsd/oneshot` 時，順帶確認了各網域的實際保護狀態：

| 網域 | Server | 受 CF 保護 |
|---|---|---|
| `hipapi1.s3file.top`（章節列表／圖片清單 API） | cloudflare | 是 |
| `m.hipmh.com`（列表／詳情頁） | cloudflare | 是 |
| `reader.hipmh.top`（閱讀器頁） | cloudflare | 是（第三個獨立 zone） |
| `hip-tx-1.s3imgs.top`（**章節內頁圖片**） | **nginx** | **否** |

**真正的漫畫內容（內頁圖片）完全不經過 Cloudflare**，所以讀圖本身永遠不會被擋。整個
第八節的問題只影響幾顆 metadata API——尤其是 `v2/chapter?hid=...`（拿圖片清單）。這解釋
了 8.1 原始症狀的形狀：「看完一話換下一話失敗」是卡在換話瞬間打的那一發 `v2/chapter`，
不是卡在載圖；manifest 一旦拿到，那一話就能完整讀完。實務意義：失敗是**點狀、瞬間**的，
重試一次通常就過，所以「清楚的錯誤訊息 + 引導使用者稍後重試」的性價比比想像中高。

**同場確認的兩件事（都是死路，不用再試）**：

- **Referer/Origin 無用**。API 回 `access-control-allow-origin: *`、沒有 `Vary: Referer`，
  站方那側完全沒有 referer 白名單；擋人的是 CF 的 `cf-mitigated: challenge`（managed
  challenge），不是站方規則。決定性反證是 8.19 本身：同一發完全沒帶 Referer 的 curl，
  16:28 被擋、16:33 就放行，請求內容一字未改——判斷依據不在 header 上。
- **`/cdn-cgi/challenge-platform/.../jsd/oneshot/...` 不是可利用的槓桿**（8.15 已查過
  `m.hipmh.com` 版本，這次確認 `reader.hipmh.top` 上是同一個機制，同樣 `405 allow: POST`）。
  它是**回報**端點不是**授權**端點：POST 上去不回傳任何通行憑證，只是把瀏覽器指紋餵給風險
  評分；payload 由 CF 自己的混淆 JS 在真實瀏覽器裡產生（canvas/WebGL/navigator/timing），
  合成不出來；URL 裡的 `nonce:timestamp:signature` 是一次性、綁定單次頁面載入的，過期即失效。
  就算硬造，從 `URLSession` 送一份「我是真瀏覽器」的指紋報告跟連線層訊號對不上，只會讓風險
  分數更差。

**前端參數對齊（`per_page`）評估後不建議改**。站方前端只用 `per_page=10`（側欄）／`20`
（章節彈窗），我們用 `50`（`html.rs:139`）。原本假設「對齊前端 → 蹭到真實使用者的熱快取」，
實測發現快取是誰打誰暖、不分身分（我自己前面的測試就把 `per_page=50` 的 page 1/2 都暖成
`HIT` 了），好處遠比預期小；而代價很明確：814 章用 `per_page=50` 只要 17 次請求，用 `20`
要 41 次。既然 8.17 判定主因是**請求量推高 IP 風險分數**，多打 24 發的傷害大於蹭快取的
好處，維持 `per_page=50` 反而正確。何況最原始的症狀 `v2/chapter?hid=...` 根本沒有參數可
對齊，冷門 hid 本來就沒人暖過。

**另確認沒有備援 API host**：前端 JS（`assets/chapters-manager.*.js`）裡只出現
`hipapi1.s3file.top` 一個 API 網域，沒有 `hipapi2`/`hipapi3` 之類可切換的鏡像，
「換 host 繞開」這條路不存在。

---

## 九、根因更正：判別變數是客戶端身分，不是 IP 或請求量（2026-09-25）

### 9.1 決定性對照實驗

使用者提供的兩個事實合起來，構成一組控制良好的對照：

1. 手機（Aidoku）與電腦接在**同一台 router**，也就是**同一個對外 IP**
2. 同一時段，**電腦瀏覽器可以連續操作漫畫更新完全不被擋**，手機每次換章都被擋

| 變因 | 手機 Aidoku | 電腦瀏覽器 | |
|---|---|---|---|
| 對外 IP | 同一個 | 同一個 | 已控制 |
| 目標主機 | `hipapi1.s3file.top` | `hipapi1.s3file.top`（頁面 XHR） | 已控制 |
| 時間 | 同一時段 | 同一時段 | 已控制 |
| 請求量 | 較少 | **較多** | 對 Aidoku 有利 |
| **結果** | **被擋** | **正常** | **不同** |

網路層變因全部相同、請求量還對 Aidoku 有利，結果卻相反。**當所有網路層變因都被控制住而
結果仍然不同，原因就不可能在網路層。** 剩下唯一在變的是「誰在發這個請求」：

| | 瀏覽器 | Aidoku |
|---|---|---|
| `cf_clearance` cookie | 有（之前解過驗證） | 無（8.8：source 自開的 WebView 用隔離 cookie store） |
| JS 偵測回報（8.15） | 持續執行 | 不執行 JS |
| TLS 指紋 | Chrome，CF 每天看幾十億次 | `URLSession`，相對少見 |

**附帶澄清**：使用者觀察到「手機跳驗證彈窗、電腦卻是直接 403」不是兩種不同的封鎖，而是
同一種封鎖打到處理方式不同的客戶端——瀏覽器只有在**導覽**時才會把驗證頁渲染出來，頁面內
的 `fetch()` 拿到 403 就只是個失敗的請求；手機那個彈窗則是 app 的 `CloudflareHandler`
主動把驗證頁轉成可見視窗（8.8）。

### 9.2 兩個被推翻的結論，以及一次推論失誤

**推翻 8.17**：「IP／行為風險分數被密集測試流量推高」解釋不了 9.1——同一個 IP 上的瀏覽器
打更多請求卻完全正常。8.19 記錄的「這次復發很可能是本 session 自己的 curl 造成的」以及
當時的自責，一併撤回：因果鏈不成立。

**推翻 8.16／8.19／8.20 對「恢復」的解讀**：那些「自己恢復」比較可能是 zone 的防護等級
隨時間浮動，或是該 URL 剛好被別的訪客暖進 CDN 快取，不是「我們的流量冷卻了」。

**一次值得記錄的推論失誤**：在確立 9.1 的診斷之後，仍一度提出「用站方書架端點判斷哪些書
有更新，藉此減少打 `hipapi1` 的請求量」的方案。使用者一句話點破：**減量在確定性模型下
無效**——17 發全失敗與 1 發失敗對使用者是同一件事，沒有任何一次會因為打得少而成功。
減量型解法只在機率性模型下成立，而那個模型（8.17）已經被自己推翻了。**教訓：推翻舊理論
之後，要回頭檢查有沒有解法還站在舊理論的地基上。**

### 9.3 目前採用的處理：講清楚失敗，不假裝能繞過

既然繞不過去，就把失敗講清楚。`src/fetch.rs` 新增：

- `check_cf_challenge(&Response)`：用 `status_code() == 403 && cf-mitigated: challenge`
  判斷（作法取自 `en.comix`，見 8.7；比比對 body 的「Just a moment」字串更權威，因為
  `cf-mitigated` 是 Cloudflare 平台標準 header，不隨挑戰版本改措辭）
- `Fetch::send` / `Fetch::json` / `Fetch::html` / `Fetch::checked`：在交給 JSON/HTML
  解析器之前先攔下驗證頁，換成「站方 Cloudflare 驗證中，請稍候幾分鐘再試」

所有打 API 的路徑都改走這些入口：`html.rs`（章節列表、圖片清單）、`json.rs`（篩選、搜尋）、
`lib.rs`（四個 HTML 頁面請求）、`home.rs`（六個平行請求，用 `Fetch::checked`）。

**同時修掉第七節記了很久的首頁容錯問題**：`home.rs::get_home()` 原本用 `popularity?` /
`korean?` / ... 逐一展開，任一分類失敗就讓整個首頁掛掉——但下方 `if !xxx.is_empty()` 的
守衛顯示原始設計意圖是「該分類單純消失」。六個分類裡有四個打的是 `hipapi1`，在第九節的
診斷下被擋是常態，所以改成：個別失敗只讓該分類消失，**六個全滅時才把第一個錯誤傳出去**
（否則使用者只會看到一片空白首頁，不知道發生什麼事）。

**這只是把錯誤訊息講清楚，不是修好。** 但依 8.21 的發現，失敗是**點狀**的（圖片 CDN
`hip-tx-1.s3imgs.top` 是裸 nginx、不經過 Cloudflare，已載入的章節讀得完，只卡在切章節
那一發 manifest），所以使用者只要知道「這是站方擋的、等一下再試」，體驗差異很大。

### 9.4 還沒做的兩個決定性實驗

後續方向取決於兩個未驗證的問題，**在有答案之前不要再寫任何繞過邏輯**：

**E1：CF 的信任綁在 cookie，還是綁在連線指紋？**（8.10 猜過是後者，從未驗證）
從正常的電腦瀏覽器複製 `cf_clearance`，用 curl 帶與不帶各打一顆**冷** URL
（`cf-cache-status` 要是 MISS 才算數）：

- 帶了成功、不帶失敗 → 綁 cookie，「把 cookie 塞進請求」可行
- 兩者皆失敗 → 綁在連線指紋等更底層，該方向死

若 E1 為正，最笨但零猜測的實作是：設定頁加一個欄位讓使用者自行貼上 `cf_clearance`，
`fetch.rs` 有值時附加 `Cookie` header（社群先例 `multi.mangadotnet` 就是手動貼，雖然是
建置期腳本）。驗證成立後再考慮用 `WebView::get_cookies()`（`js.rs:260`）自動化。
**注意**：8.5/8.6 判定 source 端 WebView 無用時，只檢查了 eval 出來的 XHR 回應，
**從來沒檢查過 `get_cookies()`**——所以「那個 WebView 到底有沒有拿到有效 cookie」其實
仍是未知，8.8 的結論可能下得太早，但也不要假設可行。

**E2：`api.s3file.top` 有沒有 hip 的內容？**（見 9.5）

### 9.4.1 實機證據：E1 大概率已經被回答了（2026-09-25）

使用者回報「點完 cf 驗證後又跳出 cf，一直重複」——症狀跟 8.10 相同，但在第九節的框架下，
這件事本身就構成 **E1 的近決定性證據**。

對照 8.8 從 host app 原始碼確認的 `CloudflareHandler.handle()` 流程：彈窗 WebView 讓
使用者手動解完 → **偵測到新的 `cf_clearance` 並橋接進 `HTTPCookieStorage.shared`** →
用 `URLSession.shared` 重打原始請求（**這一發帶著那張剛解出來的 cookie**）→ 仍被擋 →
`throw .solveFailed`。

**關鍵推論**：那個重試確實攜帶一張全新、由真人剛解出來的有效 `cf_clearance`，結果照樣
被擋。若信任綁在 cookie 上，這一步沒有理由失敗。**故 E1 的答案傾向是「綁在比 cookie 更
底層的訊號」（連線／TLS 指紋），亦即 1A（cookie 注入）大概率無效。**

「一直重複跳」的機制也一併說明：`handle()` 每次呼叫只給**一次**重試機會，失敗即結束；
下一個請求再觸發一輪全新的「彈窗 → 解 → 重試一次 → 失敗」，所以體感是無限迴圈，實際上
是多輪各自獨立的單次失敗。

### 9.4.2 E1 已定案：從原始碼驗證，cookie 有送到而且不夠（2026-09-25）

使用者質疑上述關於 app 行為的描述是否真的確認過。**確實沒有——9.4.1 以前的每一段
`CloudflareHandler` 描述都引用自 8.8，而 8.8 是更早的 session 寫的二手記載。** 因此重新
clone `github.com/Aidoku/Aidoku` 與 `github.com/Aidoku/AidokuRunner`
（後者 clone 到的 HEAD `cc4d06ff` 正好等於前者 `Package.resolved` 鎖定的 revision）逐條核對。

**8.8 的記載全部屬實**（`Aidoku/Core/Sources/Cloudflare/CloudflareHandler.swift`）：

| 8.8 的說法 | 實際位置 |
|---|---|
| `Server: cloudflare` + 403/503 + body 含 `challenge-error-title/text` 才處理 | `shouldHandle()` :69-89 |
| 0×0 隱形 WebView，`customUserAgent` 沿用原請求的 UA | `addWebView()` :163-186（:174） |
| 導覽後 3 秒、6 秒各檢查一次 captcha | :285-291 |
| 偵測到新 `cf_clearance` 就寫進 `HTTPCookieStorage.shared` | `navigated()` :295-318（:318） |
| 12 秒逾時 | :125 |
| 解完**只重試一次**，失敗即 `solveFailed` | `handle()` :101-107 |

**8.9 的 JS 運算子優先序問題在現行版本仍然存在**（`isCaptchaPage()` :385-390），但 8.10
實測彈窗會跳，所以它不是卡住的原因。

**「一直重複跳」的機制也得到證實**：`challenges[key]` 在流程結束後即被移除（:225-227），
所以下一個請求會啟動一輪全新的挑戰——新彈窗、新解、再重試一次、再失敗。體感是無限迴圈，
實際是多輪各自獨立的單次失敗。

**補上 8.8 沒查到的最後一環**——重試那一發到底有沒有帶上剛橋接的 cookie？
`Aidoku/Extensions/AidokuRunner/AidokuRunner.swift:96-119` 的 `Source.modify()`：

```swift
// add user-agent and stored cookies if not provided (for cloudflare)
let cookies = HTTPCookie.requestHeaderFields(with: HTTPCookieStorage.shared.allCookies(for: url) ?? [])
for (key, value) in cookies {
    if key == "Cookie" {
        var cookieString = value
        if let oldCookie = request.value(forHTTPHeaderField: "Cookie") {
            cookieString += "; " + oldCookie
        }
        request.setValue(cookieString, forHTTPHeaderField: "Cookie")
    }
}
```

**有，而且是明確寫死的**（不是依賴 `httpShouldHandleCookies` 預設值），註解直接寫
`(for cloudflare)`。

**完整的鏈**：使用者解開驗證 → `navigated()` :318 寫進 `HTTPCookieStorage.shared` →
`handle()` :97 呼叫 `Source.modify` 明確塞進 `Cookie` header → :101 用 `URLSession.shared`
送出（**帶著剛解出來的有效 `cf_clearance`**）→ 仍被擋 → `solveFailed`。

**結論：E1 定案——cookie 確實有送達，而且不足以放行。** 信任綁在比 cookie 更底層的訊號
（連線／TLS 指紋）。因此：

- **「把 `cf_clearance` 塞進請求」這條路正式判死**（含手動貼上、含 `WebView::get_cookies()`
  自動化）。手動貼的 cookie 不會比 app 自己送的那張更有效。
- 原本規劃的 curl 對照測試**降為非必要**，做了只是多一個獨立佐證。
- **只剩 E2／1B（換資料來源）一條路。**

**方法論教訓**：9.4.1 以前的推論全部建立在一份沒有重新驗證的二手文件上。這次核對後結論
沒變，但那是運氣，不是方法正確——**引用舊筆記描述外部程式碼行為之前，先確認它還成立。**

**附帶說明**：9.3 新加的友善錯誤訊息在這個情境下使用者**看不到**——彈窗是 app 在
`net.send()` 那一層先跳的，比 source 程式碼更早；我們的訊息要等整套 `CloudflareHandler`
放棄之後才會浮現。這不影響該改動的價值（它針對的是沒有彈窗、直接失敗的那些情況）。

### 9.5 站方帳號／書架體系的調查（`m.xipmh.com` + `api.s3file.top`）

hipmh 詳情頁裡有兩個連結指向 `https://m.xipmh.com/dashboard?lang=zh`，`aria-label="我的書架"`
——這是 hip 官方的帳號／書架入口。實測（全部未登入狀態）：

| 主機 | Server | 受 CF 保護 |
|---|---|---|
| `m.xipmh.com` | nginx/1.24.0 | **否**（Next.js，登入型 dashboard） |
| `api.s3file.top` | nginx/1.24.0 | **否** |
| `hipapi1.s3file.top` | cloudflare | 是 |

端點行為：

- `m.xipmh.com/dashboard` → 307 → `/api/auth/signin`（信箱+密碼／Google／Apple，營運方 IDX Inc.）
- `m.xipmh.com/api/auth/account` → **401 JSON**（真路由，非 SPA catch-all：亂打的路徑回 404 text/html）
- `api.s3file.top/api/v1/users/{uuid}/library` → 401「missing or invalid **authorization header**」（Bearer token，不是 cookie）
- `api.s3file.top/api/v1/manga/{uuid}` → 404「manga not found」→ **內容端點是公開的，不需登入**
- `api.s3file.top/api/v1/manga/chapters?mid=<hip 的 mid 或數字 id>` → **400「invalid manga ID」**

**關鍵未解問題（E2）**：`api.s3file.top` 用 UUID，且拒絕 hip 的每一種 ID 格式
（`bToyMzQ3NQ`、`23475`、`531490`、`17793`），傾向是**另一個獨立的 UUID 目錄**、不含
hipmh 的內容。要確認的是登入後的書架回應裡，每筆是否帶有任何能對回 hip 的識別碼
（`mid` / works slug / 指回 `m.hipmh.com` 的網址）。若有，才值得評估把資料來源整包遷移
過去——**那會是唯一能同時解決「換下一章被擋」與「書庫更新被擋」的路線**。若只有 UUID，
這條線到此為止。

**注意**：`WebLoginHandler` 登入方案（原本為了拿書架資料而設計）已隨 9.2 的減量方案一起
廢棄；而遷移路線也不需要登入，因為內容端點本來就公開。

**E2 結果（2026-09-26）：死路。** `m.xipmh.com` 是「HippaMark」個人書籤／收藏管理網站，
探索頁與書庫都要登入；`api.s3file.top` 的 `/api/v1/tags` 是公開的，但每個標籤只有約 300 本
（`mangaCount`），也沒有公開的列表端點。9.5 以為存在的 `manga/chapters?mid=` 端點其實是
`/api/v1/manga/{id}` 把 `chapters` 當成 ID 解析（`/api/v1/manga/search` 也回同一個
`invalid manga ID`）——根本沒有章節端點。不是 hip 的內容來源。

---

## 十、真正可行的路：在 source 自己的 WebView 裡發請求，用設定頁的登入視窗解驗證（2026-09-26）

> **目前解法總結（2026-09-26，已實機驗證有效）**
>
> **問題**：`hipapi1.s3file.top`（章節列表、章節圖片、篩選、搜尋）受 Cloudflare 保護。CF 只信任解過驗證
> 的那種客戶端（WKWebView），app 用 URLSession 發的請求就算帶著 `cf_clearance` 也會被擋。
>
> **解法**：
> 1. **使用者手動驗證**：來源設定裡有「Cloudflare 驗證」按鈕（`res/settings.json`，`login` / `web`，
>    網址是 `https://hipapi1.s3file.top/`）。開啟的視窗和來源的 WebView 共用同一個 per-source cookie
>    store，所以在那裡解出來的 `cf_clearance` 會存進來源自己的 store。
> 2. **請求從 WebView 發出**：`src/fetch.rs::Api` 用 `load_html_blocking` 把空白文件的網址設成 API
>    網域，然後用 `eval` 發同源同步 XHR，自動帶上那張 clearance。所有打 `hipapi1` 的地方都走 `Api`，
>    不再經過 `Request::send()`，所以 app 的 CloudflareHandler 彈窗也不會再出現。
> 3. **自動重試**：遇到 CF 挑戰時靜默重試，最多 3 次、每次間隔 1 秒；全部失敗才提示使用者去設定頁
>    重新驗證。
> 4. `m.hipmh.com`／`reader.hipmh.top` 的 HTML 頁照舊直接請求（CDN 快取，可以通過）。
>
> **使用者操作**：出現「Cloudflare 驗證已失效（已自動重試 3 次）」時，到 瀏覽 → 嬉皮 → 齒輪 →
> 「Cloudflare 驗證」，完成驗證直到畫面出現 JSON 文字，關閉視窗後再重試。不會自動跳出驗證畫面。
>
> **限制**：需要 iOS 17 以上（iOS 16 以下 `.forSource` 會退化成不共用的 `nonPersistent()` store）。
> clearance 多久會失效，要看站方的設定，目前還不知道。
>
> **不要再嘗試的方向**：改 UA、增加重試次數、減少請求量、手動貼 cookie 或用 `get_cookies` 注入、
> 看不見的 WebView 導覽到 API 讓驗證自己過、先讓 app 自動跳彈窗、換資料來源（`api.s3file.top`
> 不是 hip 的內容）。各自的理由見第八、九節與 10.1、10.6。

### 10.1 排除掉的替代資料來源

- **閱讀器頁**（`_ChapterHidPage.*.js`）：圖片清單**只有** `hipapi1/v2/chapter` 這一個來源。
  `data-api-base-url-line2` 也是 `hipapi1.s3file.top`，沒有備援 API 主機。（附帶發現：前端有一個
  `H(list, a, b)` 會用 `r.line` 等欄位算出一個 index 並 `splice` 掉一張——這是站方自己的誘餌移除
  邏輯，將來可以拿來取代六之一的 HEAD 探測。）
- **`views.s3file.top`／`w-views-1.s3file.top`／`w-views-2.xipmh.com`**（閱讀器頁的
  `stats-beacon-config`）：只是閱讀統計回報端點，而且三個都有 CF 挑戰。
- **快取對齊**：此刻用 curl 打前端**完全相同**的 URL（最熱門書、`page=1&per_page=10&order=desc`）
  也是 `403 challenge`，`hipapi1` 根網址同樣被擋。所以不能指望 CDN 快取。

所以資料來源換不掉，只能改「誰去發請求」。

### 10.2 8.8 漏掉的一環：設定頁的 web 登入視窗跟 source 的 WebView 共用同一個 cookie store

8.8 的論證是：source 的 `WebView` 用 `.forSource(key:)` 這個隔離的 store，解出來的
`cf_clearance`「寫進一個沒有任何人會讀的罐子」。**這只說對一半。** 從原始碼核對：

| 元件 | 檔案 | data store | UA |
|---|---|---|---|
| source 的 `imports::js::WebView` | `AidokuRunner/.../Utilities/WebViewHandler.swift:26-28` | `.forSource(key: id)` | 未設定（WKWebView 預設） |
| 設定頁 `"type": "login", "method": "web"` 的視窗 | `Aidoku/App/Common/Settings/WebView.swift`（`init` 裡 `config.websiteDataStore = .forSource(key: key)`） | **同一個** `.forSource(key:)` | 未設定（WKWebView 預設） |
| app 的 `CloudflareHandler` 彈窗 | `Core/Sources/Cloudflare/CloudflareHandler.swift` | `.default()` | 沿用原請求的 UA |

也就是說，**使用者在設定頁登入視窗裡手動解出來的 `cf_clearance`，source 的 WebView 會讀得到。**
（前提：`.forSource` 在 iOS 17+ 才是持久化 store，iOS 16 以下會退化成各自獨立的
`nonPersistent()`，這條路就不成立。）

### 10.3 為什麼「在 WKWebView 裡發請求」有機會過

- 8.10：使用者在 app 彈窗（WKWebView）裡解開驗證 → **CF 有接受**（拿到了 clearance）。
- 9.4.2：app 帶著這張 clearance 用 **URLSession** 重打 → 被擋。
- 所以 CF 信任的是「解驗證的那種客戶端」，而 URLSession 不是。反過來推，同一種 WKWebView
  帶著自己解出來的 clearance 發請求，條件就跟 8.10 CF 接受時一致：同一個網路堆疊、同一個 UA、
  同一個 IP、同一個 cookie store。
- 8.3–8.6 的 WebView 嘗試之所以失敗，是因為它們在一個**看不見**的 WebView 裡**導覽到 API 網址**，
  期待 managed challenge 自己過關，結果需要真人互動而卡住。這次改由使用者在**看得見**的設定頁
  視窗裡解。

### 10.4 實作

- `fetch.rs::Api`：`WebView::new()` → `load_html_blocking(空白文件, "https://hipapi1.s3file.top/")`
  把文件 origin 設成 API 網域本身（不發出網路請求）→ 每個 API 呼叫都是 `eval` 同步 XHR，屬於同源、
  第一方 cookie，也讀得到 `cf-mitigated` header。回傳 `{status, mitigated, body}`，被擋時顯示
  「請到來源設定點『Cloudflare 驗證』完成驗證後再試」。
- 所有打 `hipapi1` 的路徑都改走 `Api`：章節列表（整本共用一個 WebView）、章節圖片、篩選、搜尋、
  首頁四個 JSON 分類（共用一個 WebView）。**`hipapi1` 不再經過 `Request::send()`**，所以也不會再觸發
  app 那個「解完還是失敗」的 CloudflareHandler 彈窗。
- `res/settings.json` 新增 `login`/`web` 設定「Cloudflare 驗證」，網址是 `https://hipapi1.s3file.top/`；
  `clearCookiesOnLogOut: true` 讓「清除」按鈕真的清掉 store。
- `m.hipmh.com`／`reader.hipmh.top` 的 HTML 頁維持直接請求（CDN 長 TTL 快取，實測可通過）。
- 沒有 sleep、沒有輪詢迴圈（`zh.baka` 的教訓：WebView 內的迴圈曾讓 host app 崩潰重啟）。

### 10.5 實機驗證步驟（已於 2026-09-26 驗證成功，見 10.6）

1. **對照組**：iPhone 的 Safari 打開 m.hipmh.com 讀一話、切下一話。Safari 能過，就代表 WebKit
   本身沒被判成 bot。
2. 裝上新版 → 來源設定 →「Cloudflare 驗證」→ 在視窗內完成驗證，直到看見 JSON 文字 → 關閉視窗。
3. 回到書籍：更新章節、切換章節、搜尋。
   - 成功 → 10.3 的推論成立。要追蹤的是 clearance 多久會失效（取決於站方的 Challenge Passage 設定）。
   - 仍出現「Cloudflare 驗證已失效」→ 代表連 WKWebView 的 clearance 也不被 WKWebView 的 XHR 接受
     （例如 CF 把 clearance 綁在導覽請求上），這條路就不成立。
   - 出現「WebView 請求失敗」或「API 請求失敗（HTTP 0）」→ 是 WebView／XHR 本身的問題
     （例如 `loadHTMLString` 的 origin），跟 CF 無關，要另外查。

### 10.6 實機驗證成功 + 偶發被擋時自動重試（2026-09-26）

**使用者在實機上確認：到設定頁按「Cloudflare 驗證」完成驗證後，切章節可以正常讀取。** 10.3 的推論成立：
使用者在設定頁視窗解出來的 `cf_clearance`，在來源自己的 WebView 裡發 XHR 會被接受。這是第八、九節
所有嘗試裡第一個在實機上真正有效的方法。（第一次回報「驗證已失效」只是還沒按設定頁的按鈕，
新架構本來就不會自動跳出驗證畫面。）

**使用者要求：偶發被擋時先自動重試，都失敗才叫人去設定頁。** `Api::json` 的改法：

- 只有 `is_cf_challenge(status, cf-mitigated)` 成立才重試；200、非 CF 的 HTTP 錯誤、XHR 例外都立即處理
- 上限 `MAX_ATTEMPTS = 3`，兩次之間 `sleep(1)`，最多多等 2 秒；沿用同一個 WebView
- 3 次都被擋才顯示「站方 Cloudflare 驗證已失效（已自動重試 3 次）。請到 瀏覽 → 嬉皮 → 齒輪 →
  『Cloudflare 驗證』完成驗證後再試」
- 首頁四個 JSON 分類在全部被擋的最壞情況下，會多等 4 × 2 秒

**沒有採用「先讓 app 自動跳驗證彈窗，失敗三次再走設定頁」**：app 的 `CloudflareHandler` 彈窗把 cookie
存進 `.default()` store（來源的 WebView 讀不到），解完之後又走 URLSession 重試（9.4.2 已證實必定失敗）。
那樣會讓使用者白解三次、而且保證三次都失敗。

待觀察：clearance 多久會失效（取決於站方的 Challenge Passage 設定），以及自動重試實際救回了多少次。
