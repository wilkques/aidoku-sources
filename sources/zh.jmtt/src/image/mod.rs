use aidoku::{
    // alloc::{ String, string::ToString as _ },
    canvas::Rect, imports::canvas::{Canvas, ImageRef}, prelude::*
};

use num_traits::float::FloatCore;
use md5::{ Md5, Digest };

/// 清理圖片檔名，處理含有 `next_` 前綴的情況
///
/// 例如：`next_12345.webp?pc=xxx` → `12345`
// pub fn clean_img_filename(raw_filename: &str) -> String {
//     // 去掉 query string（? 之後的部分）
//     let without_query = raw_filename.split('?').next().unwrap_or(raw_filename);
//     // 去掉副檔名（. 之後的部分）
//     let name_without_ext = without_query.split('.').next().unwrap_or(without_query);

//     // 若包含 "next"，取底線分隔後的最後一段作為真實 ID
//     if name_without_ext.contains("next") {
//         if let Some(last_part) = name_without_ext.split('_').last() {
//             return last_part.to_string();
//         }
//     }

//     name_without_ext.to_string()
// }

/// Computes the number of slices for the image.
/// This ports the `get_num` JS function exactly.
pub fn get_pieces_num(aid_str: &str, image_id: &str) -> u32 {
    let combined = format!("{}{}", aid_str, image_id);

    // Calculate MD5 hash
    let mut hasher = Md5::new();
    hasher.update(combined.as_bytes());
    let hash_result = hasher.finalize();

    // Convert to hex string manually to get exactly what JS gets
    let hash_hex = format!("{:x}", hash_result);
    // Get last char
    let last_char = hash_hex.chars().last().unwrap();
    // Get its character code
    let char_code = last_char as u32;

    let mut n = char_code;

    let aid = aid_str.trim().parse::<u32>().expect("解析失敗");

    if aid >= 268850 && aid <= 421925 {
        n %= 10;
    } else if aid >= 421926 {
        n %= 8;
    }

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
        _ => 10, // Default fallback
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

pub fn reload_img(image: &ImageRef, pieces: u32) -> ImageRef {
    let width = image.width();
    let height = image.height() as f32;

    let mut canvas = Canvas::new(width, height as f32);

    let remainder = height % (pieces as f32);

    for i in 0..pieces {
        let mut slice_height = (height / (pieces as f32)).floor();
        let mut dst_y = slice_height * (i as f32);
        let src_y = height - slice_height * ((i + 1) as f32) - remainder;

        if i == 0 {
            slice_height += remainder;
        } else {
            dst_y += remainder;
        }

        canvas.copy_image(
            image,
            Rect::new(0.0, src_y as f32, width, slice_height as f32),
            Rect::new(0.0, dst_y as f32, width, slice_height as f32)
        );
    }

    canvas.get_image()
}
