# happymh.com 圖片 URL 逆向工程筆記

## 目標

從 `https://m.happymh.com/mangaread/weilaidegudongdian/6611564` 找出漫畫頁面圖片 URL 的組成方式。

---

## 步驟一：取得 JS Bundle

瀏覽器存成完整網頁後，主要 JS 檔案為：

```
main.656ab95a.js  (882,343 bytes)
```

---

## 步驟二：JS 混淆分析

### 混淆方式

採用 **字串陣列旋轉** 混淆（obfuscator.io 常見手法）：

```javascript
// 字串解碼函式
function D(y) {
    y = y - 0x1d9;
    return eP[y]; // eP 是旋轉後的字串陣列
}

// 初始旋轉 seed
(function(S, y) { ... }(R, 0x82948));
```

- 有效 index 範圍：`D(0x1d9)` 到 `D(0x128c)`（共 4276 個字串）
- 呼叫時會換別名：`gW(0xa85)`、`gO(0x743)` 等都是 `D` 的局部別名

### 解碼所有字串

用 Node.js 在 sandbox 裡執行 decoder：

```javascript
const vm = require('vm');
const dStart = source.lastIndexOf('function D(');
const rEnd   = source.lastIndexOf('return R();}') + 'return R();}'.length;
const decoderSrc = source.slice(dStart, rEnd);
const rotSrc = source.slice(0, source.indexOf('));', source.indexOf('}(R,0x')) + 2);
const sandbox = {};
vm.runInNewContext(decoderSrc + '\n' + rotSrc, sandbox);
const D = sandbox.D;
```

### 重要字串索引

| D(index) | 值 |
|---------|-----|
| `D(0xa85)` | `"readPage/fetchScans"` |
| `D(0x805)` | `"readPage/fetchFaiImg"` |
| `D(0x366)` | `"scans"` |
| `D(0x2ac)` | `"isEn"` |
| `D(0x7e5)` | `"url"` |
| `D(0x269)` | `"height"` |
| `D(0x10da)` | `"width"` |
| `D(0x868)` | `"/v2.0/apis/manga/reading"` |
| `D(0xe60)` | `"/apis/m/faileImg"` |
| `D(0xb70)` | `"https://jscdn.ruichuangtz.com"` |
| `D(0xe47)` | `"happymh.com"` |
| `D(0x11f5)` | `"?q="` |
| `D(0x8d9)` | `"string"` |
| `D(0x9ba)` | `"parse"` |
| `D(0xfe1)` | `"readPage"` (slice name) |

**重要發現**：`ruicdn.happymh.com` **不在** D() 字串表內，代表 CDN 的完整 URL 是由 API 回傳的，不是 client 端拼接的。

---

## 步驟三：Webpack 模組結構

Bundle 採用 Webpack 5，所有模組定義在 `S={...}` 物件中：

```javascript
S = {
  0x45a: (K, Y, X) => { /* API 端點模組 */ },
  0x49c: (K, L, Y) => { /* readPage Redux slice */ },
  0x1c63: (K, L, Y) => { /* 圖片工具函式 */ },
  0x2213: (K, L, Y) => { /* HTTP 工具 (Axios wrapper) */ },
  // ...
}
```

### 死碼陷阱

位置 442767 有一段路由設定，但被包在：

```javascript
if ('xxx' !== 'xxx') {
    // 這段永遠不執行（死碼混淆）
}
```

真正的路由在位置 778505。

### 讀取頁 Lazy Loading

讀取頁元件 `Fz` 用 React.lazy 載入，**不在主 bundle 內**：

```javascript
Fz = React.lazy(() =>
    Promise.all([
        c.e(0x2a4), c.e(0x2c8), c.e(0x168), c.e(0x122), c.e(0x2b1),
        c.e(0x225), c.e(0x1db), c.e(0x6),   c.e(0xb1),  c.e(0x3b2),
        c.e(0x134), c.e(0x51),  c.e(0x18b)
    ]).then(c.bind(c, 0x18fb))
);
```

Chunk `0x18fb` 是主要讀取元件，在執行期從伺服器動態載入，**未包含在存檔的 HTML 頁面內**。

---

## 步驟四：模組 0x45a — API 端點表

```javascript
// module 0x45a (位置 ~2600)
const base = 'https://m.happymh.com'; // 由 0x19d4 模組提供

N['CH'] = base + '/v2.0/apis/manga/reading'    // 主讀取 API
N['IM'] = base + '/apis/m/faileImg'            // 備用圖片 API
N['fM'] = base + '/v2.0/apis/manga/chapters'   // 章節列表
N['n6'] = base + '/v2.0/apis/uu/readLog'       // 閱讀記錄
N['V4'] = 'https://jscdn.ruichuangtz.com'      // GeoAPI（非圖片 CDN）
```

---

## 步驟五：模組 0x49c — readPage Redux Slice

位置約 101105，包含所有閱讀頁的狀態管理。

### fetchScans thunk（J）

```javascript
J = createAsyncThunk('readPage/fetchScans', async (FR, FD) => {
    const FS = await http.get(N['CH'], FR);  // GET /v2.0/apis/manga/reading
    if (FS.status === 0) {
        FD.dispatch(setMangaInfo({ ... }));
        FS.data.isEn
            ? FD.dispatch(j(FR))              // fetchChapters（帶完整參數）
            : FD.dispatch(j({ code: FR.code })); // fetchChapters（只帶 code）
        return FS.data;
    }
});
```

> `j` 是 `fetchChapters`，呼叫 `/v2.0/apis/manga/chapters`，**與解密無關**。

### fetchScans fulfilled reducer

```javascript
// J.fulfilled
state.isEn  = payload.isEn;
state.scans = typeof payload.scans === 'string'
    ? JSON.parse(payload.scans)  // 字串形式
    : payload.scans;             // 直接陣列
```

### successScans reducer（讀取元件呼叫）

```javascript
state.scans = [...(typeof payload.scans === 'string'
    ? JSON.parse(payload.scans)
    : payload.scans)];
```

### fetchFaiImg thunk（H）— 備用機制

```javascript
H = createAsyncThunk('readPage/fetchFaiImg', async FR => {
    const FD = await http.post(N['IM'], FR);  // POST /apis/m/faileImg
    return { index: FR.index, url: FD.data };
});

// H.fulfilled reducer
const { index, url } = payload;
if (url !== '') {
    state.scans = state.scans.map((item, i) =>
        i === index ? { ...item, url } : item
    );
}
```

---

## 步驟六：模組 0x1c63 — 圖片質量函式

```javascript
// 匯出為 ql (P 函式)
function P(url, width, height) {
    const w = Number(width), h = Number(height);
    // 手機（螢幕寬 ≤800px）用 99，桌面用 92
    const q = (window.innerWidth <= 800) ? 99 : 92;

    // Android < 11 直接回傳原 URL
    // 圖片過大（> 4096px）直接回傳原 URL

    const re = /([?&])q=\d+(&|$)/;
    if (re.test(url)) return url.replace(re, `$1q=${q}$2`);
    if (url.includes('?')) return `${url}&q=${q}`;
    return `${url}?q=${q}`;
}
```

---

## 步驟七：嘗試解密加密章節（失敗）

從存檔 HTML 內的 Redux state 取得的加密 scans 字串：

```
c4ea51af1f4d44a0ffc055c63807759109b892c23186b430521842af099e5e2f2
21a016ca68b74f94ab20c3afab5b535f8e1442f7585f6c9ef23838fbbe41d328c
f3c9727c4bca283a0f935c192f7kOA+Ia51AJ3KX38Vf+yUse+6iyZ/cEPupxEk...
```

### 分析

- 前 157 字元為 hex（78 bytes，最後 nibble 無效）
- 後半為 base64，decode 後 **2568 bytes**
- 2568 **不能被 16 整除** → 非標準 AES-CBC block 對齊

### 嘗試的解密方式（全部失敗）

| 方法 | 結果 |
|------|------|
| AES-256-CBC (key=hex[0:64], iv=hex[64:96]) | wrong final block length |
| AES-128-CBC (key=hex[0:32], iv=hex[32:64]) | wrong final block length |
| AES-CTR | 輸出亂碼 |
| XOR hex bytes with base64 bytes | 亂碼 |
| RC4 with 78-byte key | score=34，不可讀 |

### 結論

解密邏輯在 chunk `0x18fb`（動態載入），**無法從存檔頁面取得**。  
加密章節暫不支援，改用備用 API `POST /apis/m/faileImg` 逐頁取得。

---

## 步驟八：從 HTML 取得圖片 URL

嘗試直接呼叫 API 被 Cloudflare 阻擋（403 + 人機驗證）。  
改從存檔的 HTML 分析已渲染的 `<img>` 標籤。

### 發現

HTML 中存在以下結構：

```html
<div class="css-1krjvn-imgContainer">
  <div style="padding-bottom: 152.5%">space</div>
  <img referrerpolicy="origin" id="scan4"
       src="./files/49bd7cf42421769315f7adb605d15000.jpg">
</div>
```

- `id="scan{N}"` → 0-based 頁面索引
- `padding-bottom: N%` → 高/寬比 × 100（由 `scan.height/scan.width` 計算）
- `src` → 瀏覽器存檔時替換為本地路徑，原本是 CDN URL

### 核對用戶提供的範例 URL

```
https://ruicdn.happymh.com/e41f53ee97306ec9f8e4dc6e0d561148/49bd7cf42421769315f7adb605d15000.jpg?q=99
```

filename `49bd7cf42421769315f7adb605d15000` 對應 HTML 裡的 `scan4`，**完全吻合**。  
因此確認：
- **folder hash** = `e41f53ee97306ec9f8e4dc6e0d561148`（章節固定）
- **filename** = 各頁獨立的 hash（32 或 33 字元）
- **質量後綴** = `?q=99` 由 client 附加

### 存檔圖片格式驗證

```
885ce3375d7de5f92a4f04d943c691ec.jpg     : JPEG (ff d8)
2063eba655de014926f697da600ff6190.jpg    : AVIF (00 00 00 20 66 74 79 70 61 76 69 66)
2063eba655de014926f697da600ff6191.jpg    : AVIF
49bd7cf42421769315f7adb605d15000.jpg     : JPEG
```

CDN 依客戶端支援情況回傳 AVIF 或 JPEG（副檔名固定為 `.jpg`）。

---

## 最終結論

### 圖片 URL 組成

```
https://ruicdn.happymh.com/{folderHash}/{filename}.jpg?q=99
```

- `folderHash` 和 `filename` 完整包含在 API 回傳的 `scans[i].url` 欄位內
- client 端**不做任何 hash 計算**，只附加 `?q=99`
- CDN 域名 `ruicdn.happymh.com` 不在 JS bundle 裡，完全來自 API

### 取得方式

```
GET https://m.happymh.com/v2.0/apis/manga/reading?code={code}&cid={chapterId}&v=v4.300101
Referer: https://m.happymh.com/mangaread/{code}/{chapterId}
（需要登入 cookies）
```

回傳 `data.scans[]` 每項：

```json
{ "url": "https://ruicdn.happymh.com/e41f53ee.../{hash}.jpg", "width": 800, "height": 1220 }
```

### 章節 6611564 實際圖片（weilaidegudongdian）

Folder: `e41f53ee97306ec9f8e4dc6e0d561148`

| index | filename |
|-------|---------|
| 0 | `885ce3375d7de5f92a4f04d943c691ec` |
| 1 | `2063eba655de014926f697da600ff6190` |
| 2 | `2063eba655de014926f697da600ff6191` |
| 3 | `2063eba655de014926f697da600ff6192` |
| 4 | `49bd7cf42421769315f7adb605d15000` |
| 5 | `4694b71ca684a5a57b3f64d295079a1d` |
| 6 | `f5bf8937bacd885c08933f7fc1d484ea0` |
| 7 | `f5bf8937bacd885c08933f7fc1d484ea1` |

---

## 分析工具檔案（F:\works\projects\happy\）

| 檔案 | 用途 |
|------|------|
| `main.656ab95a.js` | 原始混淆 bundle |
| `main.deob2.js` | 解混淆輸出（替換所有 D() 呼叫） |
| `find_apis.js` | 解析模組 0x45a，列出所有 API 端點 |
| `find_j_func.js` | 確認 `j = fetchChapters`（不是解密函式） |
| `find_scan_decrypt.js` | 找 fetchScans 區域與 scans 使用位置 |
| `analyze_scans.js` | 分析加密 scans 字串格式 |
| `search_html2.js` | 從 HTML 找 `<img id="scan*">` 標籤 |
| `extract_all_scans.js` | 取出所有頁面的 padding-bottom 比例與檔名 |
| `verify_cdn.js` | 對 CDN 發 HEAD 請求驗證 URL（被 WAF 擋） |

---

## 步驟九：scandec.wasm 完整逆向工程（成功）

**目標**：完全逆向 `scandec.wasm`，純 Rust 實作解密，不依賴 WASM 執行環境。

### 9.1 取得 WASM 檔案

從章節 JS Chunk（存檔 HTML 內的 `395.88f8ab60.chunk.js`）找到：

```javascript
// vt._K(e, {}) 建立解密實例
const result = pako.inflateRaw(wasmDecrypt(encryptedStr, domain, flags));
```

WASM 檔案：`F:\works\projects\happy\test_scandec\scandec.wasm`（8985 bytes，AssemblyScript 編譯）

### 9.2 WASM 函式結構

用自製 WASM 二進位解析器（`disasm_all.mjs`）解析：

| 函式 | 大小 | 功能 |
|------|------|------|
| `func[35]` | 40 bytes | **匯出** `decrypt(data, domain, flags)` wrapper |
| `func[36]` | 1431 bytes | 實際解密邏輯（主體） |
| `func[27]` | 1650 bytes | SHA-256 driver（含 H0-H7 常數）|
| `func[1]` | — | AssemblyScript array byte setter |
| `func[2]` | — | AssemblyScript array byte getter |
| `func[5]` | — | 記憶體分配器 `__new(size)` |
| `func[37]` | — | Base64 decode |
| `func[8]` | — | Array byte-length getter |

**func[36] 呼叫 func[27]（SHA-256）三次**，每次用不同輸入。

### 9.3 WASM 記憶體常數

| 位移 | 內容 |
|------|------|
| 0x418 = 1048 | Secret 字串（UTF-16LE）：`DEV_SCAN_SECRET_2026_change_me` (byteLen=60) |
| 0x6D8 = 1752 | SHA-256 K constants（64 個 × 4 bytes） |

### 9.4 加密字串格式

API 回傳的加密 `scans` 字串結構（以測試章節為例）：

```
總長度 = 155 + len(base64_part)
[0..155)    hex prefix  ← 非簡單的連續 hex bytes，內部有特定結構
[155..)     base64      ← base64 decode 後得到密文（1696 bytes）
```

**hex prefix 內部佈局**（所有偏移量由 SHA-256 衍生）：

```
[0 .. off0)           gap0         (off0 = sha256[0]%24 + 16 bytes)
[off0 .. off0+64)     key1 hex     (64 hex chars = 32 bytes)
[off0+64 .. off0+64+gap1)  gap1   (gap1 = sha256[1]%24 + 8 bytes)
[off0+64+gap1 .. +32)  key2 hex   (32 hex chars = 16 bytes)
[+32 .. +gap2)         gap2        (gap2 = sha256[2]%24 + 8 bytes)
```

驗證：`off0 + 64 + gap1 + 32 + gap2 = 155` ← 恆等式成立

### 9.5 完整解密演算法

#### Step 1：SHA-256 衍生偏移量

```
sha256_result = SHA256(
    SECRET_utf8           // "DEV_SCAN_SECRET_2026_change_me"
    + encStr[0:8]_utf8    // 加密字串前 8 個 ASCII 字元
    + domain_utf8         // e.g. "happymh.com"
)

off0  = (sha256_result[0] % 24) + 16  // key1 在 hex prefix 的起始位置
gap1  = (sha256_result[1] % 24) + 8   // key1 結尾到 key2 開頭的間距
gap2  = (sha256_result[2] % 24) + 8   // key2 結尾到 base64 開頭的間距
```

#### Step 2：提取 key1 與 key2

```
key2_start = off0 + 64 + gap1
key1 = hex_decode(encStr[off0 : off0+64])      // 32 bytes
key2 = hex_decode(encStr[key2_start : +32])    // 16 bytes
```

#### Step 3：Base64 解碼

```
b64_start = key2_start + 32 + gap2  // = 155
ciphertext = base64_decode(encStr[b64_start..])  // 1696 bytes
```

#### Step 4：SHA-256-CTR 解密

每個 32 byte 區塊：

```
buf[0..32)  = key1       (32 bytes)
buf[32..48) = key2       (16 bytes)
buf[48..52) = BE32(ctr)  (4 bytes big-endian counter, starts at 0)

keystream_block = SHA256(buf)  // 32 bytes

for j in 0..32:
    output[ctr*32 + j] = ciphertext[ctr*32 + j] XOR keystream_block[j]
```

#### Step 5：驗證 magic 並解壓縮

```
// output 前 4 bytes 應為 "SC01" = [0x53, 0x43, 0x30, 0x31]
assert output[0..4] == b"SC01"

// 去掉 magic header
deflate_data = output[4..]

// raw-deflate 解壓縮（無 zlib header，即 pako.inflateRaw）
json_bytes = inflate_raw(deflate_data)

// 解析 JSON
scans: Vec<ScanItem> = serde_json::from_slice(json_bytes)
```

#### Step 6：Scan Item 結構

```json
[
  {
    "url": "https://ruicdn.happymh.com/folder_hash/filename.jpg?q=50",
    "r": 0,
    "width": 720,
    "height": 504,
    "n": 0
  },
  ...
]
```

- `url` 直接使用，不需 client 端拼接
- `r`、`n` 用途未知（目前忽略）

### 9.6 測試驗證

以測試章節為例（SHA-256 輸入 `1d39fc3e` prefix，domain `happymh.com`）：

```
sha256 = 6806cd5f51fd590cc48453d6003ea710013d965dc682f9b13eeea9344f2f3318

off0  = (0x68 % 24) + 16 = 8 + 16 = 24
gap1  = (0x06 % 24) + 8  = 6 + 8  = 14
gap2  = (0xcd % 24) + 8  = 13 + 8 = 21

key1 = aa9e4a6e41396047fe6075e1cea4e20fa9d2ab1f24ad02d1ec3356978c3f0114 (32B)
key2 = c341d3fc2231b4c31b225d878fe259ad (16B)

ciphertext[0:4] = cb bb 31 fb
keystream_block_0[0:4] = 98 f8 01 ca  (SHA256(key1||key2||00000000)[0:4])
output[0:4] = cb^98 bb^f8 31^01 fb^ca = 53 43 30 31 = "SC01" ✓

Inflated JSON: 220 scans, starts with
[{"url":"https://ruicdn.happymh.com/1a90933667b60a14a72881efdac05fb1/..."}]
```

驗證腳本：`F:\works\projects\happy\test_scandec\verify_inflate.mjs`

### 9.7 Rust 實作要點

```rust
// 需要以下 crates（已在 aidoku 框架內或需添加）:
// - sha2 (SHA-256)
// - base64
// - miniz_oxide 或 flate2 (raw deflate inflate)

fn decrypt_scans(enc_str: &str, domain: &str) -> Result<Vec<ScanItem>> {
    const SECRET: &str = "DEV_SCAN_SECRET_2026_change_me";

    // Step 1: SHA-256 衍生偏移量
    let mut hasher = Sha256::new();
    hasher.update(SECRET.as_bytes());
    hasher.update(&enc_str.as_bytes()[..8]);
    hasher.update(domain.as_bytes());
    let sha = hasher.finalize();

    let off0  = (sha[0] as usize % 24) + 16;
    let gap1  = (sha[1] as usize % 24) + 8;
    let gap2  = (sha[2] as usize % 24) + 8;
    let key2_start = off0 + 64 + gap1;
    let b64_start  = key2_start + 32 + gap2;  // = 155

    // Step 2: 提取 key1, key2
    let key1 = hex::decode(&enc_str[off0..off0+64])?;
    let key2 = hex::decode(&enc_str[key2_start..key2_start+32])?;

    // Step 3: Base64 解碼
    let ciphertext = base64::decode(&enc_str[b64_start..])?;

    // Step 4: SHA-256-CTR 解密
    let mut buf = [0u8; 52];
    buf[..32].copy_from_slice(&key1);
    buf[32..48].copy_from_slice(&key2);

    let mut output = vec![0u8; ciphertext.len()];
    for (ctr, chunk) in ciphertext.chunks(32).enumerate() {
        buf[48..52].copy_from_slice(&(ctr as u32).to_be_bytes());
        let ks = Sha256::digest(&buf);
        for (j, &b) in chunk.iter().enumerate() {
            output[ctr * 32 + j] = b ^ ks[j];
        }
    }

    // Step 5: 驗證 magic 並解壓縮
    if &output[..4] != b"SC01" {
        return Err(Error::new("decrypt magic mismatch"));
    }
    let json = inflate_raw(&output[4..])?;
    Ok(serde_json::from_slice(&json)?)
}
```

### 9.8 已使用的分析腳本

| 腳本 | 位置 | 功能 |
|------|------|------|
| `disasm_all.mjs` | `test_scandec/` | 反組譯 func[35,36,28,29,30,17] |
| `find_decrypt.mjs` | `test_scandec/` | 找到 `decrypt` 匯出 = func[35] |
| `test_crc.mjs` | `test_scandec/` | 確認 XOR 串流加密 + header 驗證 |
| `test_zero_input.mjs` | `test_scandec/` | 確認輸入格式要求 |
| `test_algo_found.mjs` | `test_scandec/` | 首次驗證 SHA-256 input = secret+data[0:8]+domain |
| `verify_algo.mjs` | `test_scandec/` | 驗證完整 key1/key2 提取 + XOR 解密 |
| `verify_inflate.mjs` | `test_scandec/` | 端對端驗證：解密 + inflate → JSON ✓ |
| `key_funcs.wat` | `test_scandec/` | func[36] WAT 反組譯（完整） |

---

## 步驟十：Rust 實作結果

### 10.1 新增檔案

**`sources/zh.happy/src/crypto.rs`**（主要實作）

純 Rust、no_std + alloc 相容，不呼叫任何 WASM 執行環境：

| 函式 | 說明 |
|------|------|
| `decrypt_scans(enc_str, domain) -> Option<Vec<ScanItemDecrypted>>` | 完整解密流程，失敗回傳 `None` |
| `sha256(data) -> [u8; 32]` | 內嵌 SHA-256（pure Rust，約 70 行） |
| `base64_decode(s) -> Vec<u8>` | 標準 base64，忽略 `=` padding 和換行 |
| `hex_decode(s) -> Option<Vec<u8>>` | 二進位 hex 解碼，長度需為偶數 |

### 10.2 修改檔案

**`sources/zh.happy/Cargo.toml`** — 新增兩個依賴：

```toml
serde_json = { version = "1.0", default-features = false, features = ["alloc"] }
miniz_oxide = { version = "0.8", default-features = false, features = ["with-alloc"] }
```

- `miniz_oxide`：raw-deflate inflate（`decompress_to_vec`），no_std + alloc 相容
- `serde_json`：直接宣告以取用 `from_slice`（與 aidoku 框架共用同一版本，無重複編譯）
- SHA-256、base64、hex decode 全部內嵌實作，**不需要額外 crate**

**`sources/zh.happy/src/lib.rs`** — 新增 `mod crypto;`

**`sources/zh.happy/src/json.rs`** — `chapter()` 函式中 `ScansValue::Encoded` 分支：

```rust
ScansValue::Encoded(enc) => {
    match crypto::decrypt_scans(&enc, "happymh.com") {
        Some(items) => items.into_iter().map(|s| ScanItem { url: s.url }).collect(),
        None => return Self::chapter_fallback(manga_key, chapter_key),
    }
}
```

解密失敗時自動 fallback 至 `POST /apis/m/faileImg`（舊方法）。

### 10.3 Build 結果

```
cargo build -p happy --target wasm32-unknown-unknown --release
→ Finished `release` profile [optimized] target(s) in 8.24s
→ happy.wasm = 256,642 bytes
```

無 error，3 個 unused import 警告（既有程式碼，與此次修改無關）。

### 10.4 待測試

- [ ] 找一個 `scans` 回傳加密字串的章節（`isEncode: true`）
- [ ] 確認解密後頁面數量正確
- [ ] 確認 URL 格式為 `https://ruicdn.happymh.com/.../xxx.jpg?q=99`
- [ ] 確認明文章節（`ScansValue::Array`）仍正常運作
- [ ] 若解密失敗（magic ≠ "SC01"），確認 fallback 不中斷

---

## 步驟十一：圖片順序錯誤除錯（2026-06-16）

### 11.1 問題描述

在 aidoku app 中開啟章節後，圖片顯示順序錯誤。曾多次修改後問題依舊。

### 11.2 原始錯誤根因：sort_by_key

舊版 `chapter()` 有這段排序：

```rust
scans.sort_by_key(|s| s.n);
```

邏輯假設：`n` 欄位是頁面閱讀順序的索引，ascending sort 可得正確排列。

### 11.3 Greasyfork 腳本驗證

找到 Greasyfork 上使用相同 SECRET 的第三方腳本，分析其行為：

- 直接使用 `scans.map(({ url }) => ...)` — **沒有任何排序**
- `n` 欄位僅用於格式驗證（確認 item 是否合法）
- **陣列原始順序 = 正確閱讀順序**

測試章節（`weilaidegudongdian/6611564`）解密後 220 個 item 全部 `n=0`、`r=0`，
代表用 `sort_by_key(|s| s.n)` 排序時，若 live API 某些章節回傳非零 `n` 值，
ascending sort 會把正確的陣列順序打亂。

### 11.4 修改清單

#### `ScanItemDecrypted`（`crypto.rs`）
移除 `n: i32`，只保留 `url: String`：
```rust
pub struct ScanItemDecrypted {
    pub url: String,
}
```

#### `ScanItem`（`json.rs`）
移除 `n: i32`，只保留 `url: String`：
```rust
pub struct ScanItem {
    pub url: String,
}
```

#### `FaileImgResponse`（`json.rs`）
移除 `n: i32`，只保留 `data: Option<String>`：
```rust
pub struct FaileImgResponse {
    pub data: Option<String>,
}
```

#### `chapter()`（`json.rs`）
移除 `sort_by_key`，直接使用陣列順序：
```rust
Ok(scans.into_iter().map(|s| Self::make_page(s.url)).collect())
```

#### `chapter_fallback()`（`json.rs`）
簡化迴圈，直接 push `Page`，移除 tuple 收集中繼結構：
```rust
match resp.data {
    Some(u) if !u.is_empty() => pages.push(Self::make_page(u)),
    _ => break,
}
```

#### `make_page()` 新增 helper（`json.rs`）
統一處理 URL 的 `?q=` 替換，避免雙重 query param（`?q=50&q=99`）：
```rust
fn make_page(raw_url: String) -> Page {
    let url = if let Some(pos) = raw_url.find("?q=") {
        format!("{}?q=99", &raw_url[..pos])
    } else if raw_url.contains('?') {
        format!("{}&q=99", raw_url)
    } else {
        format!("{}?q=99", raw_url)
    };
    Page { content: PageContent::url(url), ..Default::default() }
}
```

加密解密的 URL 已含 `?q=50`，舊邏輯直接 append 造成 `?q=50&q=99`，
現在改為找到 `?q=` 後截斷再重建。

#### `get_manga_update()`（`lib.rs`）
新增 `manga.viewer = Viewer::Webtoon;`，讓書庫已加入的漫畫在下次更新時取得正確的 Webtoon 直向閱讀模式（之前只有搜尋結果列表的 Manga 有設定）。

### 11.5 目前進度（2026-06-16）

圖片順序仍然錯誤，以下修改**均已套用**，但問題依舊：

| # | 修改 | 狀態 |
|---|------|------|
| 1 | 移除 `sort_by_key(|s| s.n)` — `n` 不是排序鍵 | ✅ 已套用 |
| 2 | 移除 `ScanItem`、`FaileImgResponse`、`ScanItemDecrypted` 的 `n: i32` 欄位 | ✅ 已套用 |
| 3 | `make_page()` 修正 `?q=` 重複問題（`?q=50&q=99` → `?q=99`） | ✅ 已套用 |
| 4 | `get_manga_update()` 新增 `manga.viewer = Viewer::Webtoon` | ✅ 已套用 |
| 5 | `chapter_fallback()` 簡化迴圈，直接 push Page | ✅ 已套用 |

### 11.6 下一步除錯方向

**根本問題仍未知**：不清楚目前走的是哪條程式路徑（明文 or 加密 or fallback）。

1. **確認 API 回傳格式**：在本機（非 Docker，不被 Cloudflare 擋）跑
   ```
   cargo test -p happy test_raw_reading_api -- --nocapture
   ```
   確認 `scans` 是明文陣列（`ScansValue::Array`）還是加密字串（`ScansValue::Encoded`）。

2. **若走加密路徑**：確認解密後陣列順序是否本身已正確（`test_get_page_list -- --nocapture` 印出 URL 順序）。

3. **若走 fallback**：`/apis/m/faileImg` POST 的 `index` 從 0 遞增，確認 API 回傳順序是否對應正確頁面。

4. **瀏覽器 DevTools**：直接觀察 `GET /v2.0/apis/manga/reading` 回應中 `scans` 陣列的原始順序，與 aidoku 顯示順序比對。

---

## 步驟十二：診斷修復計畫（2026-06-16 擬定，明日執行）

### 12.0 已釐清的事實與約束

- **錯誤樣態 = 隨機打亂**（非完全顛倒、非少數交換）→ 排除 viewer 方向與 CDN 檔名序。
- **所有漫畫都會** → 系統性根因，非單一章節資料問題。
- **我方無法取得真實 API 回應**：Docker 與本機（Windows）皆被 Cloudflare/captcha 擋。
- **aidoku `Page` 沒有 index 欄位**（`structs/mod.rs:245`，只有 `content/thumbnail/has_description/description`）
  → 顯示順序 100% 等於回傳的 `Vec<Page>` 順序，app 不會再排序。
- **解密 `crypto::decrypt_scans` 是純離線函式** → 拿到加密字串即可用 unit test 重現，繞過 Cloudflare。
- **`aidoku::println!` 輸出到 app 日誌**（設定 > 顯示日誌）→ 唯一能在「能連到 API 的環境（手機 app）」
  觀察我方程式行為的管道。

### 12.1 三個系統性根因假設

| 假設 | 說明 | 與「全部都會」契合度 |
|------|------|------|
| **(A)** 解密在 live 資料上失敗 → 每章掉進 `chapter_fallback`，`/apis/m/faileImg` 逐 index 列舉非設計用途，回傳順序亂 | 若 happymh 已全站 `isEn=true`，最能解釋 | ★★★ |
| **(B)** 解密/陣列成功，但陣列原始順序根本不是閱讀順序（存在被忽略的排序鍵；array-order 與 `n` 皆試過皆錯） | 需找出真正排序欄位 | ★★ |
| **(C)** 順序其實正確，問題在 app/viewer 端 | 「隨機打亂」使其可能性最低，仍需排除 | ★ |

### 12.2 Phase 0 — 加觀測（✅ 已實作）

已在 `json.rs` 用 `aidoku::println!` 加診斷 log（**不改邏輯**），build 通過：

- `chapter()` 進入：log `ScansValue::Array(len)` 或 `ScansValue::Encoded(len)`
- `Encoded` 分支：log `decrypted N items` 或 `decrypt failed -> fallback`
- 回傳前：log 最終 page 數與前 5 個 URL
- `chapter_fallback()`：log `entering fallback` 與每個 `index, url`、最終頁數

**取 log 流程（手機）**：
rebuild WASM → 部署到手機 app → 重新載入來源 → 開亂序章節 → 讀「設定 > 顯示日誌」→ 複製貼回。
→ 立刻得知：走哪條路徑、回傳幾頁、前幾個 URL 順序。

### 12.3 Phase 1 — 瀏覽器抓真實回應 + ground truth

已登入瀏覽器開同一亂序章節，DevTools → Network：

1. `GET /v2.0/apis/manga/reading?code=...&cid=...` 完整 JSON
   → `F:\works\projects\happy\test_scandec\real_reading_response.json`（看 `data.isEn`、`data.scans`）
2. Console 跑 ground truth：
   ```js
   [...document.querySelectorAll('img[id^=scan]')].map(e=>e.id+' '+e.src).join('\n')
   ```
   → `real_order.txt`（`scanN` 的 N = reader 頁碼）

### 12.4 Phase 2 — 離線分析（不需網路）

- **明文陣列**：比對其 url 順序 vs `real_order.txt`。相同 →(C)；不同 →(B)，列出每個 item 全部欄位找排序鍵。
- **加密字串**：在 `crypto.rs`/`test.rs` 加 `#[test]`，餵入抓到的字串字面值跑 `decrypt_scans`：
  - 失敗 →(A)：試 `domain="www.happymh.com"`、重查偏移衍生、比對 live 字串前綴/長度差異、檢查最後不滿 32B 的 CTR chunk。
  - 成功但順序 ≠ ground truth →(B)：暫時把 `ScanItemDecrypted` 加回 `r/n` 等欄位以檢視排序鍵。

### 12.5 Phase 3 — 依證據做精準修復（只做指向的那一項）

- **(A)** 修 `decrypt_scans`（domain 大小寫 / 偏移 / 長度邊界 / 末尾 chunk）；fallback 若保留須改成正確對應頁序。
- **(B)** 在 `chapter()` 對正確欄位排序，並把該欄位加回 `ScanItem`/`ScanItemDecrypted`。
- **(C)** 檢查 `get_manga_update` 的 `Viewer::Webtoon` 與 reader 設定。

### 12.6 Phase 4 — 離線回歸測試

把抓到的加密字串 + 期望順序寫成 `crypto.rs`/`test.rs` 的 `#[test]`，之後改動皆可離線驗證順序。

### 12.7 Phase 5 — app 驗證

rebuild → 重新載入來源 → 開同章節 → 對照瀏覽器順序 → 讀 app 日誌確認走對路徑。

### 12.8 明日第一步

**手機跑 Phase 0 已 build 好的版本，把日誌貼回。** 依日誌判定 (A)/(B)/(C) 後進 Phase 1～3。

---

## 步驟十三：瀏覽器 ground truth 比對 → 判定 (C)（2026-06-16）

### 13.1 取得資料（瀏覽器路線，已登入、過人機驗證）

- `real_reading_response.json`：`GET /v2.0/apis/manga/reading` 完整回應
- `real_order.txt`：reader 實際頁序（`img[id^=scan]` 依 scanN 排序的 filename）
- 兩檔位於 `F:\works\projects\happy\test_scandec\`
- 離線分析器：`F:\works\projects\happy\test_scandec\diagnose.mjs`

### 13.2 判定結果：根因 = (C)

抓到的亂序章節（`isEn:false`、`isEncode` → `scans` 為加密字串，len=10518）：

| 項目 | 結果 |
|------|------|
| 解密 | 成功，353 items（domain=`happymh.com`） |
| item 欄位 | `url, r, width, height, n`，全部 `r=0 n=0` |
| 檔名/URL 唯一性 | 353 筆全唯一，單一 folder `8b0e62bc03c8af6a44b64e6fed4a80ae`，make_page 改寫後仍 353 唯一 |
| **陣列順序 vs ground truth** | **逐一完全一致**（353/353） |

→ **我方 `chapter()` 回傳的 `Vec<Page>` 順序正確**，bug 不在資料、不在解密、不在我方排序。
→ 排除 (A)（解密成功）與 (B)（順序正確）。

### 13.3 (C) 的兩個子情況（待手機日誌分辨）

1. **App 收到正確的 353 URL 順序但顯示亂序** → 真正的 app/viewer 端 bug 或 reader 設定/快取。
2. **App 實際沒走解密路徑**（舊 build，或 in-app 解密失敗 → `chapter_fallback` 逐 index 列舉 faileImg 而亂序）。
   - 註：base_url 預設 `https://m.happymh.com`，decrypt domain 硬編 `happymh.com`（registrable domain，伺服器加密用此），離線已驗證一致 → 理論上 in-app 解密也應成功，傾向子情況 1 或舊 build。

### 13.4 下一步

**手機跑 Phase 0 build，貼回「設定>顯示日誌」。** 看 `[happy]` log：
- 有 `decrypted 353 items` + `returning 353 pages` + 正確前 5 URL → 子情況 1（app 端問題）
- 有 `decrypt failed -> fallback` 或 fallback log → 子情況 2（修 in-app 解密 / 移除 fallback）
- 完全沒有 `[happy]` log → 部署的是舊 build，需重新 build + 部署

---

## 步驟十四：手機日誌 → 確認 host-dependent payload → 修復（2026-06-16）

### 14.1 手機 Phase 0 日誌

```
[happy] chapter: ScansValue::Encoded len=10598   ← 注意：瀏覽器抓到的是 10518
[happy] chapter: decrypted 353 items
[happy] chapter: returning 353 pages
[happy]   page[0]=.../8b0e62bc.../588d24d61e3eee8f0f4483c63146e5c2.jpg?q=99
... (page[1..4])
```

→ app **解密成功、沒走 fallback、回傳 353 頁**（排除子情況 2 的 fallback 與舊 build）。

### 14.2 決定性比對

app 的 page[0..4] 檔名拿去比對瀏覽器解出的 353 筆集合 → **一個都不在裡面**。

| | host | enc len | 圖片集 | 順序 |
|---|------|---------|--------|------|
| 瀏覽器 | www.happymh.com | 10518 | A 組 | 正確（ground truth 驗證） |
| app | m.happymh.com（settings 預設） | 10598 | B 組（與 A 完全不同） | app 顯示亂序 |

→ **reading API 是 host-dependent**：同章同 folder（`8b0e62bc...`）、同樣 353 頁，但 www 與 m 回傳**不同的加密 payload**。www 的順序正確，m 的（app 用的）顯示亂序，且 m 的亂序 payload **沒有可推導的排序鍵**（檔名為隨機 hash）。

### 14.3 修復（`lib.rs` `get_page_list`）

reading API 改為 **www 優先、base host fallback**：

```rust
let json = if base.contains("//www.") {
    fetch_reading(&base, ...)?
} else {
    match fetch_reading("https://www.happymh.com", ...) {
        Ok(j) => j,                                  // www → 正確順序
        Err(_) => fetch_reading(&base, ...)?,        // www 被擋才退回 m（不會比現況差）
    }
};
```

新增 `fetch_reading(host, manga_key, chapter_key)` helper（帶對應 host 的 Referer）。
其餘 API（搜尋／章節列表／fallback）維持用 base host（m），不動已正常的部分。

build：`cargo build -p happy --target wasm32-unknown-unknown --release` ✅（僅既有 warning + `Url::chapter` 變 dead code）。

### 14.4 www 嘗試結果：失敗（已推翻、已 revert）

手機日誌：`www unreachable -> falling back to https://m.happymh.com`。app 打 www 被 Cloudflare 擋（瀏覽器有 cf_clearance、app 沒有）。**www 對 app 是死路**，14.3 的 www-first 已 revert。

---

## 步驟十五：真正根因 = 缺 `_t` cache-buster（2026-06-16）

### 15.1 推翻 host 假設

從瀏覽器抓 **m.happymh.com** 的 payload（`real_reading_m.json`）解密 → 前 5 檔名與 www 的 **完全相同（A 組）**，且 `array order == display order: true`。

→ **www vs m 不是差別**。瀏覽器打 m 拿到正確 A 組；app 打同一個 m URL 拿到亂序 B 組。差別在**請求本身**。

### 15.2 比對瀏覽器成功請求（Copy as cURL）

- **未登入**（cookie 無帳號 session）→ 排除登入因素。
- 瀏覽器請求比 app 多：`_t={ms}` 查詢參數、`avifSupport=1;webpSupport=1` cookie、`x-requested-id` header、瀏覽器 UA、`accept-language`、`cf_clearance`。
- `avifSupport/webpSupport` 只影響 **CDN 回傳的圖片位元組**（同 `.jpg` URL 回 AVIF/JPEG），**不影響 reading API 的 scans 清單** → 排除。
- 結論：最能解釋「不同檔案＋不同順序」= **缺 `_t` cache-buster**，app 打到**過期 edge 快取**拿到舊的、亂序的舊 rendering；瀏覽器永遠帶 `_t` → 當前正確版本。

### 15.3 修復（`lib.rs` `fetch_reading`）

reading 請求補上：
```rust
let ts = current_date() * 1000;   // aidoku::imports::std::current_date 回秒 → ms
// URL: .../reading?code=..&cid=..&v=v4.300101&_t={ts}
.header("X-Requested-Id", &format!("{}", ts))
.header("Accept-Language", "zh-TW,zh;q=0.9,en-US;q=0.8,en;q=0.7")
.header("User-Agent", &settings::get_user_agent())
```
**不**加 `Cookie` header（avif/webp 不影響清單，且怕覆蓋 aidoku 自動帶的 cf_clearance）。build ✅。

### 15.4 cache-buster 結果：失敗

手機日誌仍是 B 組（`588d…`），enc len 每次變但內容穩定 → `_t` 不是元兇。app 穩定拿 B 組 = 某個**穩定屬性**決定。

---

## 步驟十六：確認根因 = `avifSupport/webpSupport` cookie（2026-06-16，已解）

### 16.1 釐清：三份資料是同一章

`real_reading_response.json`、`real_reading_m.json`、app log 全是 **sishenshaonian/6619189**（folder `8b0e62bc`、353 頁）。先前誤記為 weilaidegudongdian。

→ **同一個 folder 裡同時有兩套 353 個 tile 檔**：A 組（`440bd…`，正確順序）與 B 組（`588d…`，亂序）。reading API 依客戶端**回不同清單**。

### 16.2 隔離測試（瀏覽器，決定性）

在已登入/已過 CF 的 m.happymh.com 分頁，**先刪 `avifSupport`/`webpSupport` cookie 再 fetch** reading API：

```js
document.cookie='avifSupport=; Max-Age=0; path=/';
document.cookie='webpSupport=; Max-Age=0; path=/';
fetch('/v2.0/apis/manga/reading?code=sishenshaonian&cid=6619189&v=v4.300101&_t='+Date.now(),
  {headers:{'x-requested-with':'XMLHttpRequest'}}).then(x=>x.text()).then(t=>window.__s=JSON.parse(t).data.scans);
// 然後 console 直接 copy(__s) → real_scans_nocookie.txt
```

解密結果：**B 組（`588d…`）**，與 app 完全一致。

→ **根因確認**：有 `avifSupport=1; webpSupport=1` cookie → A 組（正確）；無 → B 組（亂序 JPEG fallback）。`avif/webp` 在加密新流程裡決定的是 **scans 清單**，不只是位元組（推翻 RESEARCH 步驟八對舊流程的理解）。

### 16.3 修復（`lib.rs` `fetch_reading`）

```rust
.header("Cookie", "avifSupport=1; webpSupport=1")
```
（`_t` cache-buster 與 UA/Accept-Language/X-Requested-Id 一併保留，與瀏覽器一致、無害。）build ✅。

### 16.4 cookie header 修法結果：失敗

部署後仍 B 組。原因（Apple 文件）：URLSession `httpShouldHandleCookies=YES`（預設）會**忽略手動設的 `Cookie` header**，改用 cookie storage。aidoku 無程式化寫 cookie storage 的 API（只有 `WebView` 與 `handle_web_login`）。

---

## 步驟十七：WebView 路線（2026-06-17）

### 17.1 資料端確認無解

A 組（正確）與 B 組（亂序）是**同一套 353 切圖的不同排列**（尺寸 multiset 完全相同、13 種尺寸分佈一致），且 B 組 `r=0 n=0`、尺寸重複度高 → **B 組無任何可還原順序的資訊**。唯一解 = 讓 server 回 A 組 = 送出 avif/webp cookie。

`Accept: image/avif,image/webp` header 也試過 → 無效（server 只認 cookie）。

### 17.2 修法：reading 改走背景 WebView（`lib.rs`）

aidoku `imports::js::WebView` 寫的 cookie 會進 cookie storage（與 CF 清關同機制）。

```
fetch_reading() → 先試 fetch_reading_webview()，失敗才 fetch_reading_direct()
fetch_reading_webview:
  1. WebView::new()
  2. load_html_blocking("<html><body></body></html>", Some(host))  // 建立 m.happymh.com 原點
  3. eval: document.cookie='avifSupport=1;path=/'; ='webpSupport=1;path=/'
  4. eval: 同步 XHR GET /v2.0/apis/manga/reading?...&_t=Date.now()
          x.setRequestHeader('x-requested-with','XMLHttpRequest'); return x.responseText
  5. serde_json::from_str → ReadingApiResponse
```
同步 XHR 避開 promise；同源 + 帶 cookie → 回 A 組 JSON。build ✅。

### 17.3 WebView 各方法嘗試紀錄（依序）

**① `load_html_blocking` 建假原點（失敗）**

```rust
wv.load_html_blocking("<html><body></body></html>", Some(host))
```

`document.cookie` 在此呼叫後設定 → 不生效。WKWebView `loadHTMLString` 不會讓 `document.cookie` 寫進 cookie storage（known quirk），cookie 只活在記憶體，XHR 不帶出去 → 仍 B 組。

**② `load_blocking(api_url)` 直接載 reading API（失敗）**

```rust
let api_url = format!("{}/v2.0/apis/manga/reading?code={}&cid={}&v=v4.300101", ...);
wv.load_blocking(Fetch::get(api_url)?)?;
// 之後 eval 設 cookie + eval XHR
```

日誌顯示 `document.cookie` 確認含 `avifSupport=1;webpSupport=1`，但 XHR 仍回 B 組。  
原因：初始導向到 API endpoint 本身，cookie 雖寫進 storage，但 Cloudflare 對後續 XHR 沒有 cf_clearance → server 識別為未受信任的請求 → B 組。

**③ `load_blocking(reader_url)` 不等待（失敗）**

```rust
let reader = format!("{}/mangaread/{}/{}", host, manga_key, chapter_key);
wv.load_blocking(Fetch::get(reader)?)?;
// 立即 eval XHR（無 sleep）
```

`load_blocking` 在頁面 `load` 事件後返回，但 Cloudflare JS 挑戰腳本在 `load` **之後**才執行（challenge script 啟動時間 > load 完成時間）。所以 WebView 已有 reader 頁面，但 CF 還沒清關，XHR 仍 B 組。

**④ `load_blocking(reader_url)` + `sleep(3)` + eval cookie（成功 ✅）**

```rust
let reader = format!("{}/mangaread/{}/{}", host, manga_key, chapter_key);
wv.load_blocking(Fetch::get(reader)?)?;
aidoku::imports::std::sleep(3);   // 等 CF JS 挑戰腳本跑完
let _ = wv.eval("document.cookie='avifSupport=1;path=/';document.cookie='webpSupport=1;path=/';");
```

- `sleep(3)` 讓 CF 挑戰在背景 WebView 內完成，取得 `cf_clearance`（存入 cookie storage）
- `eval` 再補 `avifSupport`/`webpSupport` 以確保兩者都在 storage
- 後續同步 XHR 帶完整 cookie（`TREK_SESSION` 由 CF 流程自動取得，`cf_clearance` 已清關）
- Server 回 A 組（正確順序）

### 17.4 手機最終驗證（2026-06-17）✅

```
[happy] webview rendered scans: 1|https://ruicdn.happymh.com/8b0e62bc.../440bd12af0ba0ed615e70c31e5931d4f.jpg?q=99
[happy] chapter: ScansValue::Encoded len=10538
[happy] chapter: decrypted 353 items
[happy] chapter: returning 353 pages
[happy]   page[0]=https://ruicdn.happymh.com/8b0e62bc.../440bd12af0ba0ed615e70c31e5931d4f.jpg?q=99
[happy]   page[1]=https://ruicdn.happymh.com/8b0e62bc.../b642282cd9bd9caa592fcaad01474b25.jpg?q=99
...
```

- `webview rendered scans` 的 SPA DOM 自己渲染的第一張也是 A 組（`440bd…`）→ WebView 本身已被加持
- `page[0]` 也是 A 組（`440bd…`）→ XHR 拿到正確 payload
- 手機實際顯示：**圖片順序正確** ✅

### 17.5 清理診斷 log（2026-06-17）

移除所有 Phase 0 診斷用 `aidoku::println!`：`ScansValue` 型別/長度、`decrypted N items`、`returning N pages`、前 5 張 URL、`fallback index=N url=…`、`webview rendered scans`。  
保留有意義的錯誤路徑指示：`[happy] decrypt failed -> fallback`、`[happy] entering fallback`、`[happy] webview reading failed -> direct fetch fallback`。

### 17.6 最終根因與修法摘要

**根因**：happymh reading API 依 `avifSupport=1; webpSupport=1` cookie 回不同 tile 集。有 cookie → A 組（正確順序）；無 cookie → B 組（亂序 JPEG fallback，`r`/`n` 全 0 無排序鍵）。

**為何 URLSession 直接帶 Cookie header 無效**：iOS URLSession `httpShouldHandleCookies=YES`（預設）會以 cookie storage **取代**手動 Cookie header，無法從 Rust/aidoku 程式化寫入 cookie storage。

**為何需要 `load_blocking(reader)` 而非 `load_html_blocking`**：
- `load_html_blocking` → `document.cookie` 不進 WKWebView storage（loadHTMLString 的 known quirk）
- `load_blocking(api_url)` → 沒有 CF 清關，server 不信任
- `load_blocking(reader_url)` → 讓背景 WebView 真正瀏覽 reader 頁，CF 挑戰在真實頁面脈絡中完成

**為何需要 `sleep(3)`**：`load_blocking` 在 `load` event 後返回，CF JS 挑戰腳本在 `load` 後才開始執行，3 秒足夠讓挑戰完成並取得 `cf_clearance`。

**最終程式碼（`lib.rs` `fetch_reading_webview`）**：

```rust
let wv = WebView::new();
// 載入真實 reader 頁讓 CF 挑戰在 WebView 內完成（load_blocking 在 load event 返回，
// CF 挑戰腳本尚未跑完，sleep(3) 等它）
let reader = format!("{}/mangaread/{}/{}", host, manga_key, chapter_key);
wv.load_blocking(Fetch::get(reader)?)?;
aidoku::imports::std::sleep(3);
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
```

失敗時退回 `fetch_reading_direct`（純 Fetch，會拿 B 組，但至少不崩潰）。

---

## 步驟十八：改用 App 層 CF bypass + 搜尋功能（2026-06-17，進行中）

### 18.1 動機

步驟十七的背景 WebView + `sleep(3)` 方案雖正確，但每次開章節固定等 3 秒，體驗差。
目標：改用 **App 內建的 CF bypass dialog**（同 zh.baka 行為——只要前台請求打到 CF-protected URL，app 會自動彈出可見 WKWebView 讓使用者過驗證，過完 `cf_clearance` 存入共享 cookie store），讓 source 不必自己跑背景 WebView。

### 18.2 不適用的 SDK trait（已評估排除）

| Trait | 介入點 | 為何不適用 happy |
|-------|--------|------------------|
| `PageImageProcessor` | 圖片**下載後**的像素層（zh.jmtt 用於 tile 描繪重排） | happy 問題在「拿到哪組 URL 清單」，不是像素重排；B 組無排序鍵，processor 也救不回 |
| `ImageRequestProvider` | 每張**圖片下載**的 Request（zh.jmtt 為 passthrough Fetch） | happy 的 cookie 是給 reading **API** 用，不是圖片下載；時機點碰不到 |

→ 兩者皆在錯誤的層次，維持在 fetch 階段解決。

### 18.3 探索：哪個 URL 會觸發 App CF bypass dialog

| URL | 類型 | 是否觸發 bypass |
|-----|------|----------------|
| `https://m.happymh.com/`（首頁） | HTML | ❌ 不跳 |
| `{base}/latest` | JSON API | ❌ 不跳（Referer 用的，非 CF-protected） |
| `{base}/manga/{code}` | HTML 詳情頁 | 未確認 |
| `{base}/mangaread/{code}/{cid}` | HTML reader 頁 | 背景 WebView 載入時自動過（JS challenge），前台未單獨測 |
| **`{base}/sssearch`** | HTML 搜尋頁 | ✅ **唯一確認會跳出** |

### 18.4 關鍵洞察：背景 WebView vs 前台請求

**背景 WebView（`WebView::new()` + `load_blocking`）不會觸發 App 的 interactive CF bypass dialog。**

CF 有兩種挑戰：
1. **自動 JS challenge**：背景 WebView `sleep` 等一下就能自動完成 → reader page (`/mangaread/...`) 屬此類，所以步驟十七的 reading WebView 能成功。
2. **互動式 captcha（需使用者點擊）**：背景 WebView 無法自動過，靜默拿到人機驗證 HTML → `/sssearch` 屬此類。

驗證：把 search 改成背景 WebView 載 `/sssearch` + XHR，即使 `sleep(3)`，`eval` 仍回 875836B 的人機驗證 HTML（`<!doctype html>...嗨皮漫画——人机验证`）。
→ **互動式 captcha 只能靠前台 URLSession 請求觸發 App bypass dialog 讓使用者手動過。**

### 18.5 搜尋 API（POST）

happy 搜尋是 POST，非 GET query string：

```
POST {base}/v2.0/apis/manga/ssearch
Content-Type: application/json
Referer: {base}/sssearch
X-Requested-With: XMLHttpRequest

body: {"searchkey":"<漫畫名>","v":"v2.13"}
```

> 完整 body 欄位（網頁版觀察）：`searchkey`, `v: v2.13`, `s: web`（手機端不確定要送什麼）, `d:`（網頁版為空）, `page`。
> 目前精簡為只送 `{"searchkey":..,"v":"v2.13"}`，待驗證是否足夠。

`url.rs` 的 `Url::Search` variant `to_string()` 改為只回 endpoint（不含 query），query/page 在 `lib.rs` 組進 POST body：

```rust
Self::Search { .. } => format!("{}/v2.0/apis/manga/ssearch", base_url)
```

`get_search_manga_list` 用 `match &url_obj` 分流：`Url::Search` 走 POST，其餘（filter）走原本 GET。

### 18.6 當前修法（`lib.rs`，待明日手機驗證）

**Search** — 前台 POST + 偵測人機驗證頁 retry 一次：

```rust
Url::Search { query: q, .. } => {
    let body = format!("{{\"searchkey\":\"{}\",\"v\":\"v2.13\"}}", q);
    let make_request = || -> Result<String> {
        Fetch::post(url_obj.to_string())?
            .header("Content-Type", "application/json")
            .header("Referer", &format!("{}/sssearch", base))
            .header("X-Requested-With", "XMLHttpRequest")
            .body(body.as_bytes())
            .string()
    };
    let mut raw = make_request()?;
    if raw.starts_with("<!") {      // 回人機驗證 HTML → App 已彈窗，使用者過完後 retry
        raw = make_request()?;
    }
    serde_json::from_str(&raw).map_err(|_| aidoku::error!("search parse failed"))?
}
```

**Reading** — 維持背景 WebView，但 `sleep(3)` 降為 `sleep(1)`（CF bypass 已在 search 階段於前台過，cookie store 已有 `cf_clearance`，背景 WebView 載 reader 只需短暫等 JS challenge）：

```rust
wv.load_blocking(Fetch::get(reader)?)?;
aidoku::imports::std::sleep(1);
let _ = wv.eval("document.cookie='avifSupport=1;path=/';document.cookie='webpSupport=1;path=/';");
// 同步 XHR GET reading API → A 組
```

**⚠️ 18.6 前台 POST 測試結果：失敗（2026-06-17 最後一次測試）**

前台 POST + retry once 版本，手機實測 search **仍回人機驗證 HTML**（`search parse failed`，response 同樣是 `<!doctype html>...嗨皮漫画——人机验证`）。

→ 重要推論：**前台 POST（XHR-style，帶 `X-Requested-With`）似乎不會觸發 App 的 CF bypass dialog。** App bypass 可能只在「**document/navigation 類的前台 GET 請求**」（如使用者點進某頁）被 CF 擋時才彈窗，POST/XHR 被擋只是默默回人機驗證頁。
→ 這與步驟 18.3「`/sssearch` 會跳出」一致——當時觸發 bypass 的是 **GET `/sssearch`（document 導航）**，不是 POST `/ssearch`（API）。
→ 故 retry 第二次 POST 時 cookie store 仍無 `cf_clearance`，再次拿到人機驗證頁。

### 18.7 已解決：兩次 bypass

早期版本（search 前台 POST + retry 用多餘的 `Fetch::get(/sssearch).html()`）會跳兩次 bypass。
移除多餘的 `Fetch::get(/sssearch)`、直接 retry 原 POST 後，使用者確認**沒有再跳兩次**。

### 18.8 明日待驗證 / 待辦

**核心障礙**：前台 POST `/ssearch` 不觸發 App bypass dialog（見 18.6 測試結果），search 拿不到 cf_clearance。明日主攻方向：

- [ ] **讓 search 先打一個會觸發 bypass 的 document GET**（如 GET `/sssearch` 或 `/search?...`）讓 App 彈窗過 captcha，**再**打 POST `/ssearch` 拿結果。關鍵：確認這個 GET 是否真能彈窗（18.3 說會），以及過完後 POST 是否帶得到 cf_clearance。
  - 風險：早期版本用 `Fetch::get(/sssearch).html()` 做這件事曾導致「兩次 bypass」，需找到只彈一次的寫法。
- [ ] 替代方案：search 也走背景 WebView，但**先靠某個前台 GET 過 bypass**取得 cf_clearance 進共享 store，背景 WebView XHR 再帶出去（背景無法自己過互動式 captcha，但能用已存在的 cf_clearance）。
- [ ] reading 的 `sleep(1)` 是否足夠讓圖片順序正確（從 `sleep(3)` 降下來，待驗證）
- [ ] `s` 欄位手機端該送什麼值（目前省略 `{"searchkey":..,"v":"v2.13"}`，若搜尋結果異常需補 `s/d/page`）
- [ ] 確認 search 過 bypass 後，reading 是否真的不需再跳 bypass（共享 cookie store 假設）

### 18.9 關鍵事實（接手必讀）

- **背景 WebView ≠ 前台請求**：背景 WebView 不彈 App bypass dialog，只能過自動 JS challenge；互動式 captcha（`/sssearch`）必須前台 URLSession 觸發。
- **reader page = 自動 JS challenge**（背景可過）；**`/sssearch` = 互動式 captcha**（背景不可過）。
- App bypass 過後 `cf_clearance` 存入**共享 cookie store**，URLSession 與背景 WebView 共用 → 理論上一次 bypass 後兩者都受惠。
- 搜尋端點 `POST /v2.0/apis/manga/ssearch`，body `{"searchkey":..,"v":"v2.13"}`。
- 人機驗證頁特徵：response 以 `<!doctype html>` 開頭、含 `嗨皮漫画——人机验证`、約 875KB。

---

## 步驟十九：搜尋改用「背景 WebView 主、互動式 fallback」（2026-06-18）

### 19.1 重新研究新 bundle（`main.93042828.js`，Telegram 整合版）

從今日存檔的搜尋頁（`F:\works\projects\happy\漫画搜索 —— 嗨皮漫画.html` + `_files/`）反混淆：

- 字串解碼器：`function H(j,Y){...F=F-0x1da...}`，字串陣列 `M()`（index 0），rotation IIFE 自我旋轉。
  以 `vm.runInNewContext(s.slice(0, mainStart='j(),((()=>{'))` 取得 `H`，再 decode 各 token。
- 搜尋端點：`POST /v2.0/apis/manga/ssearch`（method `H(0x559)="post"`、endpoint alias `m['mo']`）。
- HTTP helper `M5(method,url,payload)`：axios config `withCredentials:true`，
  headers `Accept: application/json`、`X-Requested-With: XMLHttpRequest`、`X-Requested-Id: <ms>`；
  POST 時 `config.data = payload`（JSON body）。
- 搜尋 slice：`{name:'search', reducers:{setLastSearchValue/resetSearch/initialHistoryKeys/setHistoryKeys}}`，
  thunk `createAsyncThunk('search/fetchSearch', async(W,N)=> M5('post', m.mo, W))`。
- **W（payload）由搜尋 component 組裝，該 component 在未存檔的 lazy chunk（`src=".../js"`）內**
  → 此版 body 欄位名無法離線取得；`searchkey` 字面值在新 bundle **已不存在**（可能改名，待 DevTools 驗證）。
- 存檔搜尋頁是 React SPA，**無 server-rendered 結果**（排除「解析 HTML 取結果」這條路）。

### 19.2 採用方案（已實作於 `lib.rs`）

主用背景 WebView（沿用已驗證的 reading 機制）、失敗退互動式。`get_search_manga_list` 的
`Url::Search` 分支改呼叫 `fetch_search(base, raw_query)`：

```
fetch_search          = fetch_search_webview.or_else(fetch_search_interactive)
fetch_search_webview  : WebView::new() → load_blocking(GET {base}/mangaread/0/0)  // 觸發 CF 自動 JS 挑戰
                        → sleep(2) → eval 設 avif/webp cookie
                        → 同步 XHR POST /v2.0/apis/manga/ssearch（body 交 JS 端 JSON.stringify）
                        → 回 <! 開頭視為 CF 頁 → Err
fetch_search_interactive: 前台 GET {base}/sssearch（document 類，觸發 App 互動式 bypass dialog，只打一次）
                        → fetch_search_direct（前台 POST + 偵測 <! retry 一次）
```

- 用**原始未編碼** query 組 JSON body（`Url::Search.query` 是 `encode_uri` 版，不可用於 body）。
  新增 `escape_str()` 做 JSON/JS 字串跳脫，WebView 端只 escape 一次再靠 `JSON.stringify` 保證合法 body。
- `/mangaread/0/0` dummy 路徑：CF 規則是 path-based，edge 在 origin 404 前就發 `cf_clearance`。
  **不可**用 `/sssearch`（互動式 captcha 背景過不了，步驟 18.4 已證）。
- build ✅（`happy.wasm`，僅既有 dead-code warning）。

### 19.3 手機驗證結果（2026-06-18）✅

**主方案成功**：搜尋**靜默回結果**，無需使用者手動過驗證。

- 背景 WebView 載 `/mangaread/0/0` 觸發 CF 自動 JS 挑戰，`sleep(2)` 足夠取得 `cf_clearance`。
- 同源 XHR POST `/v2.0/apis/manga/ssearch` body `{"searchkey":…,"v":"v2.13"}` 被 CF/server 接受。
- `/mangaread/0/0` dummy 路徑確實觸發自動挑戰（edge 在 origin 404 前已發 `cf_clearance`）。
- body 欄位 `searchkey`/`v:"v2.13"` 仍有效（搜尋結果標題/封面/manga_code 正確）。
- 互動式 fallback 未被觸發（不需要每次手動驗證）。

---

## 步驟二十：搜尋再度失效，手機「直接顯示重試」（2026-07-21，進行中）

### 20.1 症狀

距步驟十九的成功驗證（2026-06-18）已一個多月。使用者回報：手機上 `get_search_manga_list`
**直接顯示重試**（畫面立即跳出錯誤/重試狀態，非等待數秒後才失敗）。目前尚未確認：

- 觸發情境是「輸入關鍵字搜尋」還是「開啟來源預設列表（無 query，走 `Url::Filter` 分支）」，
  或兩者皆然。
- 「直接」是指同步呼叫鏈本身很快回錯（例如 WebView 建立/載入立刻失敗），
  還是使用者主觀感受（未特別留意等待時間）。
- 設定 > 顯示日誌 是否有任何 `[happy]` 或錯誤訊息。

### 20.2 可能根因（待日誌驗證）

- CF 規則/挑戰腳本可能又變了（過去已發生多次：host-dependent payload、avif cookie、
  互動式 vs 自動挑戰），`/mangaread/0/0` dummy 路徑或許不再觸發自動 JS 挑戰，
  導致背景 WebView `eval` 直接拿到人機驗證 HTML，`fetch_search_webview` 快速失敗；
  接著 `fetch_search_interactive` 的前台 GET `/sssearch` 若也未彈出 bypass dialog
  （步驟 18.6 已證：非 document 導覽類請求可能不觸發），最終 `fetch_search_direct`
  同步兩次仍拿人機驗證頁 → parse 失敗 → 整體快速回傳 `Err`，UI 顯示重試。
- 也可能是 `Url::Filter`（無 query 預設列表）分支本身出錯（與 CF bypass 無關），
  例如 `/apis/c/index` 端點或參數變化、或該端點現在也被 CF 保護。

### 20.3 已加入診斷 log（`fetch.rs`，本次修改）

在 `fetch_search` / `fetch_search_webview` / `fetch_search_interactive` / `fetch_search_direct`
各關鍵點加 `aidoku::println!`（沿用步驟 12.2 的診斷模式）：

- webview 分支：建立 WebView、load 完成、eval 完成後的 body 長度
- webview 失敗時印出錯誤內容，再進 interactive
- interactive 分支：`/sssearch` GET 的回應長度或錯誤
- direct 分支：兩次嘗試各自的回應長度

`cargo build -p happy --target wasm32-unknown-unknown --release` ✅（無 error）。

### 20.4 下一步（需使用者手機操作）

1. 部署新 build 到手機 app，重新載入來源。
2. 分別測試：(a) 開啟來源預設列表（不輸入關鍵字）、(b) 輸入關鍵字搜尋。記錄兩者是否都
   「直接顯示重試」。
3. 讀「設定 > 顯示日誌」，複製所有 `[happy]` 開頭的行貼回，用以判定卡在哪個分支
   （webview / interactive / direct）以及回應長度（是否為人機驗證頁的 ~875KB 特徵）。
4. 若日誌顯示完全沒有 `[happy]` 字樣 → 代表根本沒進到 `Url::Search` 分支，問題可能在
   `Url::Filter`（預設列表）或更上游，需另外排查 `/apis/c/index`。

### 20.5 根因確定 + 修復（2026-07-22）

使用者貼出畫面錯誤原文：`Error: JsonParseError(Error("expected value", line: 1, column: 1))`。

`JsonParseError` 是 aidoku SDK `Request::json_owned()` 內部 `?` 自動轉換 `serde_json::Error`
產生的原始錯誤變體（見 aidoku-rs `imports/error.rs`）。本專案所有 search 相關的 parse 失敗都已用
`.map_err(|_| aidoku::error!("..."))` 包成自訂 `Message`，不可能顯示為 `JsonParseError`。
能讓這個原始變體外露的呼叫點只有三個未包裝的 `.json_owned()`：`lib.rs`(Filter 分支)、
`json.rs::chapters()`、`fetch.rs::fetch_reading_direct`。其中只有 **Filter 分支**
（`get_search_manga_list` 在未輸入關鍵字時走的預設列表 `/apis/c/index`）會被
`get_search_manga_list` 直接觸發，且是單次同步 fetch、無 WebView/sleep，完全符合
「直接顯示重試」（快速失敗，非等數秒才錯）。

**結論**：`/apis/c/index`（預設列表 / 篩選）現在也被 Cloudflare 擋，回人機驗證 HTML，
`json_owned()` 直接對 HTML 解析 JSON 失敗於第一個字元 → `JsonParseError("expected value", 1, 1)`。
與搜尋端點過去發生的狀況相同模式，只是換了一個端點。

**修復**（`fetch.rs` 新增 `fetch_list` / `fetch_list_direct` / `fetch_list_webview` /
`fetch_list_interactive`，`lib.rs` 的 `_ =>` 分支改呼叫 `fetch::fetch_list(&base, url_obj.to_string())`）：

1. `fetch_list_direct`：先照舊直接 `Fetch::get + string()`，多一層「以 `<!` 判斷 CF 頁」再解析，
   避免多數仍可行的情況下多繞 WebView。
2. 失敗（含 CF 頁）→ `fetch_list_webview`：沿用 search 已驗證的背景 WebView 套路
   （`load_blocking({base}/mangaread/0/0)` 觸發自動 JS 挑戰 → `sleep(2)` → 設 avif/webp cookie
   → 同步 XHR GET 實際 list URL）。
3. 仍失敗 → `fetch_list_interactive`：前台 GET `{base}/sssearch`（已確認會觸發 App 互動式
   bypass dialog）過驗證後，retry 一次 `fetch_list_direct`。

加上與 search 相同風格的 `[happy]` 診斷 log。`cargo build -p happy --target
wasm32-unknown-unknown --release` ✅ 無 warning。

### 20.6 手機驗證：列表已修好，換閱讀頁報錯（2026-07-22）

部署後**預設列表恢復正常**（20.5 的修法有效）。但使用者接著回報開漫畫內頁（`get_page_list`）
出現：

```
[zh.happy] Error: JsonParseError(Error("missing field `scans`", line: 1, column: 66))
```

**分析**：同樣是原始未包裝的 `JsonParseError`，代表命中 `fetch_reading_direct`
（`fetch_reading_webview` 的 parse 失敗有 `.map_err` 包成自訂訊息，不可能顯示成
`JsonParseError`）。`fetch_reading_direct` 原本也是直接 `.json_owned()` 無包裝，
同一種模式的 bug 再現一次(reading 版本)。

`line 1, column 66` 代表回應是一段**只有 66 字元的合法 JSON**，但沒有 `data.scans` 欄位
——不是 CF 人機驗證 HTML（那是 `<!doctype...`，不會是合法 JSON），比較可能是 API 本身
回傳的錯誤/狀態物件（例如需要登入、參數錯誤、章節不存在等），實際內容目前未知。

**已做的修正**（`fetch.rs`）：
- `fetch_reading`：webview 失敗時加 `[happy] reading webview failed: ... -> trying direct` log。
- `fetch_reading_webview`：eval 後印 body 長度；parse 失敗訊息改附上內容前 200 字元
  （原本只印長度，看不到實際錯誤內容）。
- `fetch_reading_direct`：先取 `.string()` 印長度、parse 失敗改用 `serde_json::from_str`
  + 自訂 `aidoku::error!` 包裝並附前 200 字元，不再是裸 `.json_owned()`。
- 新增 `truncate_str()` helper：安全截斷到最近的 UTF-8 char boundary，避免中文內容在
  200 byte 處被從多 byte 字元中間切斷而 panic。

`cargo build` ✅ 無 warning。**尚未手機驗證**。

### 20.7 下一步

部署新 build，重新開同一本漫畫的同一章節，讀「設定 > 顯示日誌」：
- 有 `[happy] reading webview failed: ...` → 代表 webview 分支本身出錯（load/eval 失敗），
  接著看 `reading direct parse failed (len=66): <內容>` 的實際內容判斷是登入/CF/參數問題。
- 若沒有 webview failed log，但看到 `reading direct: body len=66` → 代表這次 webview
  成功但 direct 分支不該被呼叫；需重新檢查 `fetch_reading` 呼叫順序（理論上 webview
  成功不會走到 direct，除非日誌次序造成誤判，需連 webview 的 log 一起看）。
- 這次 error 訊息本身現在應該已經包含 66 字元的實際內容（不只是長度），若使用者能連同
  完整錯誤訊息一起貼出即可直接判斷根因，不一定需要另外翻日誌。

### 20.8 拿到實際內容 → 根因 = 未先造訪漫畫詳情頁（2026-07-22）

日誌與錯誤訊息帶出實際回應（webview 與 direct 兩分支結果一致，長度都 197）：

```json
{"status":400,"data":{"redirect":"/manga/miewangzhihoudeshijie"},"msg":"页面已过期，返回漫画详情页重新获取章节"}
```

翻譯：「頁面已過期，返回漫畫詳情頁重新獲取章節」。伺服器要求先造訪 `/manga/{code}`
（漫畫詳情頁）才能核發/更新有效的章節閱讀 token，`fetch_reading_webview` 與
`fetch_reading_direct` 原本都是直接跳去 `/mangaread/{code}/{cid}` 或直接打 reading API，
跳過了這一步。

**修復**（`fetch.rs`）：
- `fetch_reading_webview`：在載入 reader 頁之前，先在同一個 WebView 內 `load_blocking`
  `{host}/manga/{manga_key}`（漫畫詳情頁），再照舊載 reader 頁 → sleep → 設 cookie → XHR。
- `fetch_reading_direct`：呼叫 reading API 之前，先 `Fetch::get({host}/manga/{manga_key})`
  取一次（結果忽略，只為讓伺服器核發 token；`Set-Cookie` 走 URLSession cookie storage，
  不需要 JS 就能生效，這點與 avif/webp 那次不同——那次是要用 JS `document.cookie` 寫，
  這次伺服器本身用標準 HTTP `Set-Cookie` response header，直接 GET 就會被瀏覽器/URLSession
  自動存起來）。

`cargo build` ✅ 無 warning。**尚未手機驗證**：需要確認多造訪一次詳情頁是否真的解掉
「頁面已過期」，以及是否會拖慢每次開章節的速度（多一次網路請求，webview 分支多一次
`load_blocking`）。

### 20.9 回報結果與新版本完全相同 → 補 log 判斷是否真的測到新版（2026-07-22）

使用者測試後回報的日誌與 20.6 的內容**逐字相同**（同樣的 200/197 長度、同樣的 400 錯誤），
但當時的修法（先載 `/manga/{code}` 詳情頁）**沒有對應的 log 行**，所以無法從這份日誌判斷：

1. 是使用者忘記重新編譯/部署，測到的其實還是舊版；還是
2. 新版真的有跑，多造訪一次詳情頁**沒有解決**過期問題（例如：token 是詳情頁 SPA 完成
   JS 渲染/呼叫某內部 API 後才核發，而非單純載入 HTML 就有；`load_blocking` 只等到
   `load` 事件，可能早於 SPA 真正核發 token 的時間點——類似 CF 挑戰腳本在 `load` 後
   才跑完的先例）。

**已補強**（`fetch.rs`）：
- `fetch_reading_webview`：detail 頁 `load_blocking` 前後各加一行 log
  （`loading detail {url}` / `detail load done`），reader 頁 load 完也加一行。
- `fetch_reading_direct`：detail 頁 GET 的結果加 log（`detail page len=N` 或錯誤）。

`cargo build` ✅ 無 warning。

### 20.10 下一步

部署這個新版後重新測試同一操作，這次日誌應該會多出：
```
[happy] reading webview: loading detail https://.../manga/{code}
[happy] reading webview: detail load done
[happy] reading webview: reader load done
```
（direct 分支則是 `reading direct: detail page len=N`）

- 若日誌**還是完全沒有這些新行** → 確定沒測到新版，需要重新確認編譯產物是否真的部署上去
  （檢查 wasm 檔案時間戳、來源是否有重新載入）。
- 若**有**這些新行、但錯誤依舊 → 代表「造訪詳情頁」本身不足以核發 token，下一步猜測方向：
  在 detail 頁 load 後加 `sleep(1~2)` 讓 SPA 有時間執行完初始化 API 呼叫再導去 reader 頁
  （尚未實作，先觀察此輪日誌再決定）。

### 20.11 確認測到新版，但問題依舊 → 加 sleep 讓 SPA 有時間跑（2026-07-22）

新日誌證實這次確實測到新版（多了 `loading detail` / `detail load done` / `reader load
done` 三行，且 direct 分支的 detail page 長度是 232165，是真的載到完整頁面），但錯誤
訊息與長度（197、400、同一個 redirect）**仍然一模一樣**。

**排除**：不是「沒測到新版」，也不是「detail 頁沒載到內容」。

**新假設**：`load_blocking` 只等到初始 HTML shell 的 `load` 事件，detail 頁的 SPA
（React lazy chunk）真正執行、呼叫核發 token 用的內部 API，是在 `load` 之後才發生
——與步驟十七「CF 挑戰腳本在 `load` 後才跑完，需要 `sleep(3)`」是同一種模式。目前
detail 頁 load 完之後**沒有任何等待**就立刻導去 reader 頁，SPA 很可能根本沒機會執行。

**已修改**（`fetch.rs`，僅 webview 分支；direct 分支不執行 JS，sleep 對它無意義）：
detail 頁 `load_blocking` 完成後加 `sleep(2)`，才繼續導去 reader 頁。加了對應 log
(`detail load done, sleeping for SPA init`)。`cargo build` ✅。

**尚未手機驗證**。若這輪加了 sleep 後 webview 分支還是同樣的 400/197，代表問題不是
「SPA 沒跑完」，需要往別的方向找（例如：`/manga/{code}` 這條路由本身就是通用 SPA
shell、不含 manga-specific 資料擷取邏輯；或核發 token 的關鍵其實是呼叫
`chapterByPage`/`chapterByPage` 系列 API 本身，而不是造訪任何 HTML 頁面——下一步可
嘗試在 reading 之前顯式呼叫一次 `/v2.0/apis/manga/chapterByPage?code={manga_key}&page=1`
而非（或加上）造訪 `/manga/{code}`）。

### 20.12 sleep(2) 無效 + 排除單一漫畫問題 → 找到真正根因：API 版本號過期（2026-07-22）

**排除步驟**：
1. 加了 `sleep(2)` 後重測，日誌確認新版有跑（多了 `sleeping for SPA init`），但錯誤
   訊息、長度(197)、redirect **仍然完全相同** → 推翻「SPA 需要時間跑完」假設。
2. 換另一部完全不同的漫畫測試，**結果一樣** → 排除「特定漫畫下架/遷移」假設，確認是
   系統性問題，影響所有漫畫的閱讀請求。

**關鍵觀察**：章節列表 API（`chapterByPage`，`url.rs` 的 `Url::Book`）**沒有帶 `v=` 版本
參數**且運作正常；閱讀 API 卻寫死帶 `v=v4.300101`（步驟九從 2026-06 中旬存檔頁面取得的
版本字串），而閱讀 API 一律失敗。懷疑伺服器現在會檢查閱讀 API 的 `v=` 版本號，過期版本
一律視為「頁面過期」擋下。

**驗證**：請使用者在電腦瀏覽器登入 `m.happymh.com`、開一個章節、F12 Network 面板抓實際
請求，結果：

```
https://m.happymh.com/v2.0/apis/manga/reading?code=miewangzhihoudeshijie&cid=6693949&v=v4.300102&_t=1784651206180
```

**確認**：目前線上版本是 `v4.300102`，我方寫死的 `v4.300101` 已過期一個版本號。這正好
解釋「所有漫畫的閱讀請求全部失敗、但不帶版本號的列表類 API 都正常」這個現象——伺服器對
帶過期版本號的閱讀請求，回的就是這個通用「頁面已過期，請返回詳情頁重新整理」訊息，
跟造訪詳情頁/CF/token 完全無關，之前 20.5~20.11 的造訪詳情頁 + sleep 修法都是誤打的方向
（雖然對根因無效，但也無害，暫時保留在程式碼裡沒有拿掉，之後若確認版本號修好即可、
且想省一趟額外的 network round trip，可以再移除）。

**修復**：全部改成 `v=v4.300102`
（`src/fetch.rs` 的 webview XHR URL 與 direct URL、`src/json.rs` 註解、`src/test.rs` 的
`test_raw_reading_api`）。`cargo build` ✅ 無 warning。**尚未手機驗證**。

**⚠️ 重要提醒**：這個版本號是網站前端建置版本，未來還會再變，屆時同樣的「頁面已過期」
錯誤會再出現。若之後再遇到同樣症狀（所有漫畫閱讀都失敗、列表正常），**第一步就該先比對
`v=` 版本號**，不用再重跑一輪 CF/token 假設。

### 20.13 清掉誤打方向的修改（2026-07-22）

版本號才是真正根因後，把 20.5～20.11 那些針對「造訪詳情頁核發 token」假設做的修改全部
還原（`fetch_reading_webview`/`fetch_reading_direct`）：

- 移除 `/manga/{manga_key}` 詳情頁的預先造訪（webview 版與 direct 版都刪）
- 移除對應的 `sleep(2)`（等 SPA 初始化）
- 移除該假設專用的診斷 `println!`（loading detail / detail load done / reader load done /
  reading direct: detail page len 等）

**保留**（跟版本號無關，本身是好的修正，不因假設被推翻而失效）：
- `fetch_reading_direct` 改用 `.string()` + 自訂錯誤包裝並附內容前 200 字元
  （不再是裸 `.json_owned()`，這正是這次能一路追出版本號問題的關鍵——若是裸錯誤，
  只會看到通用 `JsonParseError`，看不到伺服器實際回傳的 400 內容）
- `truncate_str()` 安全截斷 helper
- `fetch_reading` 頂層在 webview 失敗時印 `[happy] reading webview failed: ... -> trying
  direct`（沿用整份檔案「只保留有意義的錯誤路徑指示」慣例，其餘逐步驟診斷 log 移除）

`cargo build` ✅ 無 warning。目前 `fetch_reading_webview`/`fetch_reading_direct` 的結構
已恢復成步驟十七/十九驗證過的原始邏輯，只差版本號從 `v4.300101` 改成 `v4.300102`。

### 20.14 清理後仍卡 loading（2026-07-22）

使用者部署 20.13 的乾淨版本後回報：**仍然卡在 loading，跟清理前一樣**。這代表卡住的
原因跟已經拿掉的「造訪詳情頁 + sleep」無關，是別的地方卡住(可能是 `load_blocking`
本身、`eval` 同步 XHR 沒有 timeout 卡住、或其他)。

**已加入逐步 checkpoint log**（`fetch.rs`），在 `fetch_reading_webview` 的
建立 WebView / `load_blocking` 前後 / `eval` 前後，以及 `fetch_reading_direct` 的
GET 前後都加 `aidoku::println!`，這樣即使最後卡住沒有回傳，也能從已印出的最後一行
log 判斷卡在哪一步。`cargo build` ✅。

**下一步**：部署後重現卡 loading，去「設定 > 顯示日誌」看目前印出的最後一行
`[happy] reading webview: ...` 或 `reading direct: ...`，貼給我判斷卡在
建立 WebView / 載入 reader 頁 / eval 同步 XHR 的哪一步。

### 20.15 日誌揭曉：reading API 已修好，卡在解密失敗 → 慢速 fallback（2026-07-22）

使用者貼出的日誌關鍵內容（含一則系統層 `[ERROR]`，跟 `[happy]` log 混在一起，時間順序
上 `[ERROR]` 是最後發生的）：

```
[happy] reading webview: creating WebView
[happy] reading webview: load_blocking https://m.happymh.com/mangaread/miewangzhihoudeshijie/6693949
[happy] reading webview: load_blocking returned, sleeping
[happy] reading webview: calling eval
[happy] reading webview: eval returned, body len=4130
[ERROR] ... NSURLErrorTimedOut ... URL=https://m.happymh.com/apis/m/faileImg
```

**判讀**：
- `eval returned, body len=4130` 且**沒有**接在後面的 `webview reading parse failed`
  訊息 → reading API 呼叫本身**成功了**（20.12 的版本號修法確認有效！JSON 解析過關）。
- 卡住/逾時的網址是 `https://m.happymh.com/apis/m/faileImg`——這是 `json.rs`
  `chapter_fallback()` 逐頁 POST 的舊備援端點，**只有 `crypto::decrypt_scans` 解密
  失敗**才會走到這裡。
- 結論：reading API 沒問題了，但 `scans` 是加密字串時，`decrypt_scans` 解密**失敗**，
  掉進 `chapter_fallback` 逐頁（可能上百頁）同步 POST 的慢速迴圈，才會感覺「卡
  loading」，最終某一頁 timeout 報錯。

**已加診斷 log**：
- `crypto.rs::decrypt_scans`：在每一個回傳 `None` 的分支前加 log（過短 / offset 算出來
  太大 / key1 or key2 hex_decode 失敗 / ciphertext 太短 / magic 不是 "SC01" / inflate
  失敗 / JSON parse 失敗），並在成功時印出解析到幾個 items。
- `json.rs::chapter()`：解密失敗、要進 `chapter_fallback` 前加一行
  `decrypt failed -> entering fallback`。

`cargo build` ✅ 無 warning。**尚未手機驗證**。

### 20.16 下一步

部署後重現同樣操作，這次應該會在 `eval returned, body len=4130` 之後多出一串
`[happy] decrypt: ...` 的診斷行，把完整內容貼給我。重點看：
- `off0/gap1/gap2/key2_start/b64_start` 有沒有算出明顯不合理的值（例如 b64_start
  超過字串長度、或跟 enc len 對不太上）
- 卡在哪一步回傳 `None`（hex_decode / ciphertext 太短 / magic mismatch / inflate 失敗 /
  json parse 失敗）
- 如果卡在 `magic mismatch`，代表 SHA-256-CTR 解出來的東西不對，可能是
  `SECRET`（`"DEV_SCAN_SECRET_2026_change_me"`）已經隨網站更新輪替，或者 domain 字串
  （目前寫死 `"happymh.com"`）需要改成別的值——這兩個都要等這輪日誌出來再決定要
  往哪個方向查。

### 20.17 日誌 + 現場 SECRET 萃取 → 根因確認 + 修復（2026-07-22）

使用者這輪直接貼出完整診斷 log：

```
[happy] decrypt: enc len=3736
[happy] decrypt: off0=17 gap1=18 gap2=31 key2_start=99 b64_start=162
[happy] decrypt: ciphertext len=2680
[happy] decrypt: output[0..4]=138 170 226 141
[happy] decrypt: magic mismatch (expected SC01)
[happy] chapter: decrypt failed -> entering fallback
```

offset 公式算出來的值全部合理（b64_start 與 enc_len/ciphertext_len 換算對得上），
排除「格式跑掉」，鎖定金鑰材料（SECRET）本身可能已隨網站更新輪替。

**現場驗證步驟**：
1. 使用者提供一個真實 `GET reading` API 的完整 JSON 回應（電腦瀏覽器截取，manga_code=
   `miewangzhihoudeshijie`、cid=`6693949`），內含真實加密 `scans` 字串。
2. 使用者另外提供目前線上版本的 `scandec.wasm`（存成 base64 data URI，
   `F:\works\projects\happy\test_scandec\real_scans_warm.txt`，來源不明確但內容確認是
   目前版本的 wasm，10385 bytes，比步驟九當時的 8985 bytes 略大）。
3. 用 Node.js 掃描 wasm 的資料段找 UTF-16LE 字串（沿用步驟九同款手法），在 offset 9285
   找到（AssemblyScript byteLength 欄位確認長度 86 bytes = 43 字元，完整無截斷）：
   ```
   PRO_SCAN_SECRET_20260712_watching_you_DEBUG
   ```
   對照舊的 `DEV_SCAN_SECRET_2026_change_me`——**SECRET 字串本身被輪替了**
   （`DEV`→`PRO`，日期戳從 `2026` 變 `20260712`，後綴也變了），offset 公式、domain
   （`happymh.com`）皆未變。
4. 用離線 Node script（`verify_new.mjs`，沿用步驟九 `verify_algo.mjs` 邏輯）拿使用者提供的
   真實加密字串 + 新 SECRET + domain=`happymh.com` 跑一遍：
   `magic="SC01"` ✓，`inflateRawSync` 成功解出 42282 bytes 的 JSON，
   內容為合法的 `[{"url":"https://ruicdn.happymh.com/...jpg?q=50",...}, ...]` 陣列。
   **完整驗證成功**，證實只有 SECRET 變了，演算法/offset 公式/domain 全部不變。

**修復**：`crypto.rs` 的 `SECRET` 常數改成
`b"PRO_SCAN_SECRET_20260712_watching_you_DEBUG"`。`cargo build` ✅。

**新增離線回歸測試**（`test.rs::test_decrypt_scans_after_secret_rotation`，
`#[aidoku_test]`）：把這次使用者提供的真實加密字串內嵌進測試，直接呼叫
`crypto::decrypt_scans` 斷言解密成功且第一筆 URL 以 `https://ruicdn.happymh.com/` 開頭。
此測試不需網路，未來 SECRET 若再輪替，這個測試會立刻紅燈，不用等手機回報。
（本機沙箱沒有 `aidoku-test-runner` 執行環境，只驗證了 `cargo test -p happy --no-run`
能編譯成 wasm test 二進位；實際跑測試需在使用者自己裝好 runner 的環境執行
`cargo test -p happy -- --nocapture`。）

### 20.18 完整結論（2026-07-22）

這次「閱讀頁空白/卡 loading」totally 是兩個獨立的網站端更新疊加：

1. reading API 的 `v=` 版本號從 `v4.300101` 過期到 `v4.300102`（步驟 20.12）。
2. `scandec.wasm` 的解密 `SECRET` 從 `DEV_SCAN_SECRET_2026_change_me` 輪替成
   `PRO_SCAN_SECRET_20260712_watching_you_DEBUG`（本步驟）。

兩者都修好後，`fetch_reading` 應該能直接拿到解密成功的分頁，不需要再掉進慢速的
`chapter_fallback`。**待手機最終驗證**：重新部署後開章節應該能正常顯示圖片、
不再卡 loading。若還有問題，一樣看 `[happy]` log（這次應該會看到
`decrypt: parsed N items` 而不是 `magic mismatch`）。

**⚠️ 未來提醒**：SECRET 字串本身的命名（`PRO_SCAN_SECRET_{日期戳}_watching_you_DEBUG`）
暗示網站作者會不定期輪替這個常數，日期戳可能就是輪替版本的線索。以後如果又遇到
「reading API 成功但 decrypt 一路 magic mismatch」，直接重複本節做法：抓一份目前
`scandec.wasm`（瀏覽器 DevTools 存成 base64 或直接下載 .wasm 檔）、掃 UTF-16LE 字串
找新 SECRET，不用重跑整套 WASM 反組譯（步驟九的 offset 公式、domain 值目前為止都還沒變過）。

### 20.19 手機驗證成功 + 清理診斷程式碼（2026-07-22）

使用者確認：漫畫內容恢復正常顯示。整個步驟二十（列表 CF bypass + reading API 版本號 +
crypto SECRET 輪替）全部驗證完成。

清理本次除錯過程中加入、但後來證實與根因無關或已無必要的診斷 log：

- `fetch_reading_webview`/`fetch_reading_direct`：移除 20.14 為了排查「卡 loading」加的
  逐步 checkpoint log（creating WebView / load_blocking 前後 / calling eval / eval
  returned / GET 前後），只留 `fetch_reading` 頂層失敗轉移那一行
  （`reading webview failed: ... -> trying direct`，符合步驟 17.5 的既有慣例：
  只留有意義的錯誤路徑指示）。
- `crypto.rs::decrypt_scans`：移除 20.15 加的所有逐步診斷 log（enc len、offset 值、
  ciphertext len、output 前 4 bytes、magic mismatch、inflate 失敗、json parse 失敗、
  成功筆數），完全恢復成步驟十的乾淨版本，只有 `SECRET` 常數值不同。
- `fetch_search_webview`/`fetch_search_interactive`/`fetch_search_direct`：移除本次
  除錯一開始（誤以為問題出在搜尋）加的逐步診斷 log（creating WebView、loading、
  load done、eval done、first/retry attempt len），恢復成步驟十九驗證過的乾淨版本。

**保留**（跟根因無關但本身是好的、永久性的修正）：
- `fetch_list`（`lib.rs` 預設列表分支的 CF bypass 修法，20.5 的核心修復）
- `fetch_reading_direct`/`fetch_reading_webview` 的錯誤訊息改良（`.string()` +
  `truncate_str` 內容預覽，取代裸 `json_owned()`——這是這次能一路追出版本號問題的
  關鍵能力，永久保留）
- `fetch_reading`/`fetch_search`/`fetch_list` 頂層策略切換時的單行失敗指示 log
  （只在失敗轉移下一個策略時觸發，不是逐步 checkpoint）
- `json.rs::chapter()` 的 `decrypt failed -> entering fallback` 單行指示
- `crypto.rs` 的 `SECRET` 新值
- `test.rs::test_decrypt_scans_after_secret_rotation` 離線回歸測試

`cargo build -p happy --target wasm32-unknown-unknown --release` ✅ 無 warning。

### 20.20 本次除錯總結（2026-07-22）

一次回報「get_search_manga_list 手機上直接顯示重試」，實際上牽出網站端**三個獨立、
同時間疊加**的變動：

1. `/apis/c/index`（預設列表）現在也被 Cloudflare 擋 → 修法：`fetch_list`
   （direct 優先 → 背景 WebView 過自動挑戰 → 前台互動式 bypass）。
2. reading API 的 `v=` 版本號從 `v4.300101` 過期到 `v4.300102`。
3. `scandec.wasm` 解密用的 `SECRET` 從 `DEV_SCAN_SECRET_2026_change_me` 輪替成
   `PRO_SCAN_SECRET_20260712_watching_you_DEBUG`。

診斷方法論回顧（供未來參考）：
- 手機日誌（`aidoku::println!` + `設定 > 顯示日誌`）是本機/Docker 被 Cloudflare 擋掉時
  唯一能觀察「連得到 API 的環境」實際行為的管道。
- 錯誤訊息務必包一層自訂內容（含長度、前 N 字元預覽），裸 `json_owned()`/`?` 洩漏的
  `JsonParseError` 只講「格式不對」不講「內容是什麼」，會浪費好幾輪來回才問到關鍵字串。
- 電腦瀏覽器（已登入、已過 CF）可以直接用 DevTools Network/Sources 抓到手機端因
  Cloudflare/互動式驗證而拿不到的真實資料（reading API 回應、`scandec.wasm` 本體），
  離線用 Node.js 重建同一套演算法驗證，比在 app 裡來回猜測快得多。
- 每次深挖後，若假設被推翻（如本次的「造訪詳情頁核發 token」），要主動清掉對應的
  程式碼與診斷 log，避免下一輪除錯被無關的歷史修改干擾判斷。

---

## 步驟二十一：能否重複使用 CF token + reading/search 效能與正確性調整（2026-07-22~23）

### 21.1 起因：能否把 cf_clearance「取出來」重複使用？

使用者提問：WebView 過 Cloudflare 挑戰成功後，能不能把 token 取出來讓外部重複使用，
藉此避免每次都重新跑一次 WebView 挑戰？

**調查結論（查 aidoku-rs SDK 原始碼，`imports/js.rs` + `imports/net.rs`）**：

- `WebView` 只有 `new/load/load_blocking/load_html/load_html_blocking/wait_for_load/eval`
  幾個方法，**沒有任何讀取 cookie 的 API**，唯一管道是 `eval("document.cookie")`。
- `cf_clearance` 是 `HttpOnly` cookie，`document.cookie` 對 JS 完全隱形，**結構上讀不到**。
- SDK 也沒有 `CookieJar`/cookie store 讀取介面，唯一相關的是 `WebLoginHandler::
  handle_web_login`——host app 主動推 cookie 給 source 的 callback（給登入流程用），
  不是我們能主動查詢的 pull API。
- 手動 `Request::header("Cookie", ...)` 早在步驟 16.4 就驗證過會被 iOS URLSession
  的 cookie storage 機制忽略。

**結論**：無法程式化「取出再塞回去」。但不需要——`cf_clearance` 本來就是透過標準
`Set-Cookie` response header 在**原生網路層**（URLSession/WKWebView 共用同一個系統
cookie storage）自動存好的，不經過 JS。只要 WebView 導航一次成功過關，之後任何
`Request`（包含我們自己的 direct fetch）理論上都能自動吃到，不需要手動搬運。

### 21.2 嘗試：reading 改 direct-first（失敗，已 revert）

依 21.1 的結論，嘗試把 `fetch_reading` 從「webview 優先」改成「direct 優先」，
省去每次開章節都跑一次 WebView 導航 + `sleep(1)`。build 通過，但**手機測試後圖片
順序回歸錯亂**。

**根因**：圖片順序正確與否綁定在 `avifSupport`/`webpSupport` cookie 上（步驟十六
結論），這兩個 cookie 只有 webview 用 `document.cookie` 才寫得進去，`fetch_reading_
direct` 完全没有機制設定（手動 Cookie header 會被忽略）。`cf_clearance` 常態有效，
改 direct-first 後幾乎每次都直接解析成功——但成功拿到的是**沒有 avif/webp cookie
的亂序資料（B 組）**。`cf_clearance` 有效與「圖片順序正確」是兩件獨立的事，不能混為
一談。已 revert 回 webview-first，並在程式碼註解記下這個教訓，避免重蹈覆轍。

### 21.3 搜尋再度出現 CF 擋（2026-07-22）

使用者回報搜尋出現：

```
[happy] search webview failed: Message("search webview hit CF challenge (len=875921)")
Error: Message("search parse failed (len=875836)")
```

875KB 正是人機驗證頁特徵長度。第一個假設：觸發用的假路徑 `/mangaread/0/0` 可能被
CF 判定得比之前嚴格。改成已知存在的真實章節 `weilaidegudongdian/6611564`（與
reading 觸發自動挑戰的方式一致，新增常數 `CF_TRIGGER_PATH`）——**測試結果：無效**，
換真實路徑後長度仍是同一個 ~875KB 驗證頁，排除「假路徑判定變嚴」這個假設。

互動式 fallback（前台 GET `/sssearch` 觸發 App bypass dialog）理論上使用者過關後
`cf_clearance` 應該共用，但後續 POST 仍然拿到相同驗證頁。使用者確認畫面**有跳出**
驗證視窗——代表 App bypass 機制本身正常，問題出在别的地方。

### 21.4 抓包比對→發現 body 格式整個錯了（2026-07-22）

請使用者在電腦瀏覽器 DevTools 用「Copy as fetch」抓一次真實搜尋請求，結果：

```js
fetch("https://m.happymh.com/v2.0/apis/manga/ssearch", {
  headers: {
    "content-type": "application/x-www-form-urlencoded",
    "x-requested-id": "1784734947677",
    "x-requested-with": "XMLHttpRequest",
    // ...sec-ch-ua-*, sec-fetch-* 等瀏覽器自動帶的 header
  },
  referrer: "https://m.happymh.com/sssearch",
  body: "searchkey=%E5%8F%8D%E8%BD%AC%E7%BB%83%E4%B9%A0&v=v2.13&s=web&d=",
  method: "POST",
  credentials: "include",
});
```

**關鍵發現**：真實請求是 `application/x-www-form-urlencoded`，body 是
`searchkey=...&v=v2.13&s=web&d=` 四個欄位；我方程式碼一路以來（從步驟十八開始）
送的都是 `application/json` 的兩欄位 JSON body `{"searchkey":...,"v":"v2.13"}`，
少了 `s`、`d`，Content-Type 也整個錯誤。步驟 18.5 當時就已經記錄「待驗證是否足夠」，
這次終於確認**不夠**。

**修復**（`fetch.rs`）：

- 新增 `form_url_encode()`（等同 JS `encodeURIComponent`，逐 UTF-8 byte 編碼，
  對照抓包的 `反轉練習` percent-encode 結果驗證編碼規則正確）。
- 新增 `build_search_body(query)` 統一組出
  `searchkey={encoded}&v=v2.13&s=web&d=`。
- `fetch_search_webview`（XHR）與 `fetch_search_direct`（原生 POST）都改用這個
  body + `Content-Type: application/x-www-form-urlencoded`。
- 移除不再需要的 JSON 專用跳脫函式 `escape_str`。
- 同步更新 `test.rs::test_raw_search_api` 離線診斷測試。

隨後使用者又貼一張瀏覽器 Network 面板截圖，補上兩個程式碼漏掉的 header：
`Accept: application/json`、`X-Requested-Id: {timestamp_ms}`。
截圖裡 Cookie 那串（`TREK_SESSION`/`_dvid`/`_dcs`/`cf_clearance`/`avifSupport`/
`webpSupport`/`_ga_*`）全部是共享 cookie store 自動帶的，不需要手動處理；
`Sec-Ch-Ua-*`/`Sec-Fetch-*` 是瀏覽器引擎自動加的，WebView 路徑會自動帶，
native Request 路徑無法複製但非必要（見 21.5 的結論）。

### 21.5 真正根因：webview XHR 結構性不可能通過 Referer 檢查（2026-07-23）

body 格式修好後，使用者回報：連續搜尋兩次都有結果顯示，但**每次都跳一次
Cloudflare 驗證視窗**——功能正常但體驗很差。加上內容預覽的診斷 log 後拿到關鍵訊息：

```
[happy] search webview failed: Message("search webview parse failed (len=51):
  {\"status\":400,\"data\":[],\"msg\":\"not support\",\"u\":[]}") -> trying interactive
[happy] interactive: /sssearch after bypass len=875556 starts_with_challenge=true
[happy] direct: attempt 1 len=12197 starts_with_challenge=false
```

`{"status":400,"msg":"not support"}` 是**伺服器真的收到請求、明確拒絕**，不是
CF 擋（不是 `<!` 開頭）。

**根因**：伺服器額外檢查搜尋請求的 `Referer` 必須是 `/sssearch`。`fetch_search_
webview` 在背景 WebView 裡用 XHR 發送，`Referer` 由瀏覽器自動帶成 WebView 當時
載入的頁面網址（`CF_TRIGGER_PATH` 章節頁），**JS 規範明確禁止手動覆寫 `Referer`
（forbidden header name）**，所以這個 XHR 的 Referer 永遠對不上，伺服器永遠回
`"not support"`。這是結構性限制，不是哪裡沒設對——`fetch_search_webview` 這條路
**本質上不可能通過**這個檢查，每次都白白浪費一次 WebView 建立 + 導航 + `sleep(2)`，
然後必然失敗、掉到互動式 fallback，導致每次搜尋都要跳一次驗證視窗。

而 `fetch_search_direct`/`fetch_search_interactive` 用原生 `Request`，可以自由
設定 `Referer: {base}/sssearch`，天生就滿足這個檢查；只要 `cf_clearance` 有效
就會成功——這正是為什麼互動式 fallback 之後 `direct: attempt 1` 幾乎都成功的原因。

**修復**（`fetch.rs`）：整條 `fetch_search_webview` 移除，`fetch_search` 改成
direct-first：

```rust
pub fn fetch_search(base: &str, query: &str) -> Result<ApiResponse> {
    match fetch_search_direct(base, query) {
        Ok(resp) => Ok(resp),
        Err(e) => {
            aidoku::println!("[happy] search direct failed: {:?} -> trying interactive", e);
            fetch_search_interactive(base, query)
        }
    }
}
```

`build` ✅ 無 warning（`CF_TRIGGER_PATH` 仍被 `fetch_list_webview`/
`fetch_reading_webview` 使用，不是死碼）。

### 21.6 手機驗證：連續搜尋不再跳窗（2026-07-23）✅

使用者連續搜尋 6 次，日誌全部是：

```
[happy] direct: attempt 1 len=... starts_with_challenge=false
```

沒有任何一次出現 `search direct failed`，代表 `cf_clearance` 從第一次（或更早的
list/reading 操作）取得後持續有效，後續搜尋全部靜默透過 direct 成功，**不再需要
反覆跳出 Cloudflare 驗證視窗**。

### 21.7 本輪結論與教訓

1. **`cf_clearance` 讀不出來、也不需要讀出來**——它是原生網路層自動管理的
   HttpOnly cookie，SDK 沒有存取 API，也沒必要有，因為背景 WebView 與 native
   Request 本來就共用同一份系統 cookie storage。
2. **「CF clearance 有效」≠「拿到的資料是對的」**——reading 的圖片順序正確性
   綁定在 JS 可寫的 `avifSupport`/`webpSupport` cookie 上，跟 CF 挑戰是兩件
   完全獨立的事，direct-first 對 reading 是錯的優化方向（21.2）。
3. **XHR/fetch 規範裡的 forbidden header（如 `Referer`）是背景 WebView 方案的
   硬性天花板**——只要伺服器檢查這類欄位，背景 WebView 內發的請求就永遠無法
   滿足，不用再花時間調整 sleep 時間或觸發路徑，直接改用能自訂 header 的原生
   `Request`。
4. **搜尋/清單類 API 適合 direct-first**（cheap，且不像 reading 有 cookie
   依賴的正確性問題）；**reading 必須 webview-first**（正確性依賴 JS 寫入的
   cookie）。這個不對稱是本輪最容易搞混、也最容易走回頭路的地方，已在
   `fetch.rs` 對應函式加上明確註解說明「不可改成 direct-first」的原因。
5. 診斷方法論再次印證：瀏覽器 DevTools「Copy as fetch」/ Network 截圖，能在
   幾分鐘內揪出「body 格式整個錯了」這種本機/Docker（被 CF 擋）完全無法察覺的
   問題，比反覆猜測 CF 規則快得多。
