use aidoku::{ alloc::{ String, string::ToString as _ }, prelude::* };
use base64::{ engine::general_purpose, Engine };

/// 清理圖片檔名，處理含有 `next_` 前綴的情況
///
/// 例如：`next_12345.webp?pc=xxx` → `12345`
pub fn clean_img_filename(raw_filename: &str) -> String {
    // 去掉 query string（? 之後的部分）
    let without_query = raw_filename.split('?').next().unwrap_or(raw_filename);
    // 去掉副檔名（. 之後的部分）
    let name_without_ext = without_query.split('.').next().unwrap_or(without_query);

    // 若包含 "next"，取底線分隔後的最後一段作為真實 ID
    if name_without_ext.contains("next") {
        if let Some(last_part) = name_without_ext.split('_').last() {
            return last_part.to_string();
        }
    }

    name_without_ext.to_string()
}

/// 計算圖片應被切成幾片（對應 JS 的 get_num 函數）
///
/// JS 原始呼叫方式：
/// ```js
/// var num = get_num(btoa(aid), btoa(img.id.split(".")[0]));
/// ```
///
/// 注意：JS 傳入的是 base64 編碼後的值
pub fn get_pieces_num(aid: &str, img_id: &str) -> u32 {
    let aid_int: u32 = aid.parse().unwrap_or(0);

    // 舊版圖片固定切 10 片
    if aid_int < 268850 {
        return 10;
    }

    // JS: get_num(btoa(aid), btoa(img_id))
    // 先 base64 編碼，再串接後計算 md5
    let b64_aid = general_purpose::STANDARD.encode(aid.as_bytes());
    let b64_id  = general_purpose::STANDARD.encode(img_id.as_bytes());
    let combined = format!("{}{}", b64_aid, b64_id);
    let digest   = md5::compute(combined.as_bytes());
    let hash_str = format!("{:x}", digest);

    let last_char = hash_str.chars().last().unwrap_or('0');
    let mut n = last_char as u32;

    // 依 aid 範圍決定取模基數
    if aid_int >= 268850 && aid_int <= 421925 {
        n %= 10;
    } else if aid_int >= 421926 {
        n %= 8;
    }

    // 將餘數對應到實際片數（2、4、6 … 20）
    match n {
        0 => 2,
        1 => 4,
        2 => 6,
        3 => 8,
        4 => 10,
        5 => 12,
        6 => 14,
        7 => 16,
        8 => 18,
        9 => 20,
        _ => 10,
    }
}

/// 從頁面的 <script> 標籤中提取 infiniteScrollConfig 的某個欄位值
///
/// 支援 `key: "value"`、`key: 'value'`、`key: 123` 三種格式
pub fn extract_js_config<'a>(script_text: &'a str, key: &str) -> Option<&'a str> {
    let search = format!("{}: ", key);
    let start = script_text.find(&search)? + search.len();
    let rest = &script_text[start..];

    // 判斷是單引號、雙引號還是無引號
    let first = rest.chars().next()?;
    if first == '"' || first == '\'' {
        // 字串值（單引號或雙引號）
        let inner = &rest[1..];
        let end = inner.find(first)?;
        Some(&inner[..end])
    } else {
        // 數字/布林值
        let end = rest.find(|c: char| !c.is_alphanumeric() && c != '.')?;
        Some(&rest[..end])
    }
}
