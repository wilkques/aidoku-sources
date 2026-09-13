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

## 七、還沒驗證 / 已知風險

- **章節圖片解密**：本機 `aidoku-test-runner` 因為 Windows 工具鏈缺 `dlltool.exe` 跑不起來（`cargo install aidoku-test-runner` 會失敗），沒辦法在 wasm 環境裡實際端到端測過；目前的信心來自「用 Node 直接執行混淆碼＋比對 Rust 手刻邏輯的中間值」，邏輯上應該等價，但還沒有手機上實機確認章節圖片能正常顯示（進行中）。
- **`cargo test` 對 `m.hipmh.com` 的網路請求會失敗**（`RequestError`），研判是 `reqwest`（test-runner 用的 client）的 TLS 指紹被 Cloudflare 擋掉，`curl`／真機不受影響；純選擇器/解碼邏輯已經全部改用不連網的 fixture 離線測試繞過這個限制，見 `src/test.rs`（目前整份被註解，需要用時解開特定測試即可）。
- `get_page_list` 之外，`Url::book`/`Url::chapter` 已修正，但 home.rs 舊版 `.mh-item`/`.mh-item-tip` 選擇器邏輯（服務首頁用）已經整段刪除，改直接共用 `GenManga::list()`。
- 章節圖片 CDN 有 line1/line2 兩條線路可以切換（站方前端存 localStorage），目前固定吃頁面當下給的 `data-chapter-img-base`（line1 預設），沒有實作切線路的容錯機制。
