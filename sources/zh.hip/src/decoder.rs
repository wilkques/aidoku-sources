use aidoku::alloc::{String, Vec};
use serde::Deserialize;

// 章節內頁圖片：reader.hipmh.top 的頁面不含明文圖片網址，是打
// GET {api_url}/v2/chapter?hid={api_hid} 拿一段被打亂/編碼過的字串，
// 前端用 /assets/runtime/chapter-decoder.js（重度混淆）解開。
// 演算法已用真實資料逐步還原（見對話記錄），流程：
//   1. 去掉頭尾標記 "qM9"..."Z7"
//   2. 剩餘內容切成 尾/頭/雜訊 三段（用內部的 "Vx"/"pL0" 標記分隔），
//      重新接成 尾+頭+雜訊
//   3. 每 7 字一組，奇數組（0-based）反轉
//   4. 用自訂字母表當 base64 字母表解出 bytes（bit 順序跟標準 base64url 相同，
//      只是字母順序被打亂）
//   5. UTF-8 decode 後就是一段 JSON 字串陣列（圖片相對路徑）
const CIMG_ALPHABET: &[u8; 64] = b"_-9876543210abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const CIMG_PREFIX: &str = "qM9";
const CIMG_SUFFIX: &str = "Z7";
const CIMG_MARKER1: &str = "Vx";
const CIMG_MARKER2: &str = "pL0";
const CIMG_CHUNK: usize = 7;

#[derive(Deserialize)]
pub struct ChapterImagesResponse {
    pub data: ChapterImagesData,
}

#[derive(Deserialize)]
pub struct ChapterImagesData {
    pub images: String,
}

pub fn decode_chapter_images(input: &str) -> Option<Vec<String>> {
    let bytes = input.as_bytes();

    if !input.starts_with(CIMG_PREFIX) || !input.ends_with(CIMG_SUFFIX) {
        return None;
    }

    let core = &bytes[CIMG_PREFIX.len()..bytes.len() - CIMG_SUFFIX.len()];

    if core.len() <= CIMG_MARKER1.len() + CIMG_MARKER2.len() {
        return None;
    }

    let rem = core.len() - CIMG_MARKER1.len() - CIMG_MARKER2.len();
    let tail_len = rem / 3;
    let remainder = rem - tail_len;
    let head_len = remainder / 2;
    let junk_len = remainder - head_len;

    let head = &core[0..head_len];
    let marker1_start = head_len;
    let marker1 = &core[marker1_start..marker1_start + CIMG_MARKER1.len()];
    let junk_start = marker1_start + CIMG_MARKER1.len();
    let junk = &core[junk_start..junk_start + junk_len];
    let marker2_start = junk_start + junk_len;
    let marker2 = &core[marker2_start..marker2_start + CIMG_MARKER2.len()];
    let tail_start = marker2_start + CIMG_MARKER2.len();
    let tail = &core[tail_start..];

    if marker1 != CIMG_MARKER1.as_bytes()
        || marker2 != CIMG_MARKER2.as_bytes()
        || tail.len() != tail_len
    {
        return None;
    }

    let mut combined = Vec::with_capacity(tail.len() + head.len() + junk.len());
    combined.extend_from_slice(tail);
    combined.extend_from_slice(head);
    combined.extend_from_slice(junk);

    let mut shuffled = Vec::with_capacity(combined.len());

    for (i, chunk) in combined.chunks(CIMG_CHUNK).enumerate() {
        if i % 2 == 1 {
            shuffled.extend(chunk.iter().rev());
        } else {
            shuffled.extend_from_slice(chunk);
        }
    }

    let mut bits: u32 = 0;
    let mut nbits: u32 = 0;
    let mut out = Vec::with_capacity(shuffled.len() * 3 / 4);

    for c in shuffled {
        let value = CIMG_ALPHABET.iter().position(|&a| a == c)? as u32;

        bits = (bits << 6) | value;
        nbits += 6;

        if nbits >= 8 {
            nbits -= 8;
            out.push((bits >> nbits) as u8);
        }
    }

    let json_str = String::from_utf8(out).ok()?;

    serde_json::from_str::<Vec<String>>(&json_str).ok()
}

// 解碼出來的路徑陣列偶爾會在同一個頁碼相鄰出現兩筆（例如
// "..._46.2gi6me.webp" 緊接著 "..._46.w5v1n8.webp"）。實測其中一筆一律是站方塞的
// 誘餌：回應 200 但 Content-Type 是 image/png（內容只是幾十 bytes 的 1x1 透明圖，
// 沒有 Etag/Cache-Control 長期快取），另一筆才是真正的 webp 內頁。**哪一筆是誘餌
// 沒有固定順序**——同一本書不同章節試過「誘餌在前」跟「誘餌在後」都出現過，不能單靠
// 陣列順序判斷，只能實際打一次請求看 Content-Type（見 html.rs 的
// resolve_decoy_duplicates，用 HEAD 請求判斷後再從頁面清單裡踢掉誘餌那筆）。
// 這裡只提供「取出檔名裡的頁碼」這個共用邏輯給 html.rs 用來分組。
pub(crate) fn page_number_key(path: &str) -> Option<&str> {
    let filename = path.rsplit('/').next()?;
    let stem = filename.split('.').next()?;

    stem.rsplit('_').next()
}
