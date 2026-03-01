use aidoku::{
    canvas::Rect,
    imports::{
        canvas::{Canvas, ImageRef},
        std::current_date,
    },
    prelude::*,
};

use md5::{Digest, Md5};
use num_traits::float::FloatCore;

pub fn get_image_pieces_num(aid_str: &str, image_id: &str) -> u32 {
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

pub fn reload_image(image: &ImageRef, pieces: u32) -> ImageRef {
    let width = image.width();
    let height = image.height() as f32;

    let mut canvas = Canvas::new(width, height as f32);

    let remainder = height % (pieces as f32);

    for i in 0..pieces {
        let mut slice_height = FloatCore::floor(height / (pieces as f32));
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
            Rect::new(0.0, dst_y as f32, width, slice_height as f32),
        );
    }

    canvas.get_image()
}

pub fn get_current_day_of_week() -> i64 {
    let current_timestamp = current_date() as i64;
    let local_timestamp = current_timestamp + 8 * 3600; // UTC+8
    let days_since_epoch = local_timestamp / 86400;
    let day_of_week = (days_since_epoch + 3) % 7 + 1; // 1 = Monday, 7 = Sunday
    day_of_week
}
