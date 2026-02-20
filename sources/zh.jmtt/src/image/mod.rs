use aidoku::{ alloc::{ String, string::ToString as _ }, prelude::* };

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

/// 根據 aid 與圖片檔名計算圖片應被切成幾片
///
/// 切片數規則（移植自 JM 網站 JS）：
/// - aid < 268850：固定 10 片
/// - aid 268850~421925：md5 最後字元 % 10 對應偶數片數
/// - aid >= 421926：md5 最後字元 % 8 對應偶數片數
pub fn get_pieces_num(aid: &str, img_filename: &str) -> u32 {
    let aid_int: u32 = aid.parse().unwrap_or(0);

    // 舊版圖片固定切 10 片
    if aid_int < 268850 {
        return 10;
    }

    // 計算 md5(aid + 圖片名) 的最後一個十六進位字元
    let combined = format!("{}{}", aid, img_filename);
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
