use aidoku::{
    alloc::{String, Vec}
};

use base64::{engine::general_purpose, Engine as _};

// 定義一個結構來裝載解析後的結果
#[derive(Debug, PartialEq)]
pub struct ScrambleData {
    pub width: f32,
    pub height: f32,
    pub args: Vec<[f32; 8]>, // 儲存切割座標
}

/// 【純函數】：將 Base64 字串解析成寬、高與座標陣列
/// 這個函數完全不依賴 Aidoku 的 API，所以可以 100% 被 cargo test 執行
pub fn decode_jm_canvas_data(base64_str: &str) -> Option<ScrambleData> {
    // 1. 解碼 Base64
    let decoded_bytes = general_purpose::STANDARD.decode(base64_str).ok()?;
    let json_str = String::from_utf8(decoded_bytes).ok()?;

    // JSON 格式長這樣: {"url":"...","args":[[sx,sy,sw,sh,dx,dy,dw,dh],...],"width":720,"height":500}
    
    // 2. 擷取 width
    let width_str = json_str.split("\"width\":").nth(1)?.split(',').next()?.trim();
    let width: f32 = width_str.parse().ok()?;

    // 3. 擷取 height
    let height_str = json_str.split("\"height\":").nth(1)?.split('}').next()?.trim();
    let height: f32 = height_str.parse().ok()?;

    // 4. 擷取 args 陣列
    // 取出 [[...],[...]] 的部分
    let args_part = json_str.split("\"args\":[").nth(1)?.split("],\"").next()?;
    
    let mut args = Vec::new();
    // 每個區塊看起來像 [0,415,720,85,0,0,720,85]
    for arg_group in args_part.split("],[") {
        // 去除可能殘留的括號
        let clean_group = arg_group.replace('[', "").replace(']', "");
        
        let nums: Vec<f32> = clean_group
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
            
        if nums.len() == 8 {
            let mut arr = [0.0; 8];
            arr.copy_from_slice(&nums[..8]);
            args.push(arr);
        }
    }

    Some(ScrambleData { width, height, args })
}