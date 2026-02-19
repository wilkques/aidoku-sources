#![expect(clippy::unwrap_used)]

use super::*;
use aidoku_test::aidoku_test;

// #[aidoku_test]
// fn test_get_search_manga_list() {
//     let source = Jmtt::new();

//     let filters = vec![
//         FilterValue::Select {
//             id: String::from("類型"),
//             value: String::from("hanman"), 
//         },
//         // FilterValue::Select {
//         //     id: String::from("题材"),
//         //     value: String::from("性感"), 
//         // },
//         // FilterValue::Select {
//         //     id: String::from("地区"),
//         //     value: String::from("2"), 
//         // },
//         // FilterValue::Select {
//         //     id: String::from("进度"),
//         //     value: String::from("0"), // 全部
//         // },
//     ];

//     let result = source
//         // .get_search_manga_list(Some("富家女".to_string()), 1, Vec::new())
//         // .get_search_manga_list(None, 1, Vec::new())
//         .get_search_manga_list(None, 1, filters)
//         .unwrap();

//     panic!("完整結果: {:#?}", result);
// }

// #[aidoku_test]
// fn test_get_manga_update() {
//     let source = Jmtt::new();

//     // 1. 建立一個假的 Manga 用於測試
//     let manga = Manga {
//         key: "1111972".to_string(), // 換成真實的漫畫 ID 以測試
//         title: "与初恋的意外同居 / 與初戀的意外同居".to_string(),
//         cover: Some("https://cdn-msp3.18comic.vip/media/albums/1111972_3x4.jpg?u=1771167758".to_string()),
//         url: Some(
//             "https://18comic.vip/album/1111972".to_string(),
//         ),
//         ..Default::default()
//     };

//     // 2. 傳入正確的三個參數
//     let result = source
//         .get_manga_update(manga, true, true)
//         .unwrap();

//     panic!("完整結果: {:#?}", result);
// }

// #[aidoku_test]
// fn test_get_page_list() {
//     let source = Jmtt::new();

//     let manga = Manga {
//         key: "1111972".to_string(),
//         title: "与初恋的意外同居 / 與初戀的意外同居".to_string(),
//         cover: Some("https://cdn-msp3.18comic.vip/media/albums/1111972_3x4.jpg?u=1771167758".to_string()),
//         ..Default::default()
//     };

//     let chapter = Chapter {
//         key: "1111972".to_string(),
//         title: Some("第1話".to_string()),
//         chapter_number: Some(1.0),
//         ..Default::default()
//     };

//     let result = source
//         .get_page_list(manga, chapter)
//         .unwrap();

//     panic!("完整結果: {:#?}", result);
// }

// #[aidoku_test]
// fn test_handle_deep_link() {
//     let source = Jmtt::new();

//     // 測試案例 1: 有效的網址
//     let valid_url = "https://18comic.vip/album/1111972/%E4%B8%8E%E5%88%9D%E6%81%8B%E7%9A%84%E6%84%8F%E5%A4%96%E5%90%8C%E5%B1%85-%E8%88%87%E5%88%9D%E6%88%80%E7%9A%84%E6%84%8F%E5%A4%96%E5%90%8C%E5%B1%85".to_string();

//     let result = source.handle_deep_link(valid_url).unwrap();

//     panic!("✅ 成功解析 DeepLink: {:?}", result);
// }

#[aidoku_test]
fn test_decode_jm_canvas_data() {
    // 這是從你提供的網頁 HTML 中複製下來的真實 canvas data
    let raw_base64 = "eyJ1cmwiOiJodHRwczovL2Nkbi1tc3AyLjE4Y29taWMudmlwL21lZGlhL3Bob3Rvcy8xMTExOTcyLzAwMDAxLndlYnA/cGM9MTc3MDAwNDAzMCIsImFyZ3MiOltbMCw0MTUsNzIwLDg1LDAsMCw3MjAsODVdLFswLDMzMiw3MjAsODMsMCw4NSw3MjAsODNdLFswLDI0OSw3MjAsODMsMCwxNjgsNzIwLDgzXSxbMCwxNjYsNzIwLDgzLDAsMjUxLDcyMCw4M10sWzAsODMsNzIwLDgzLDAsMzM0LDcyMCw4M10sWzAsMCw3MjAsODMsMCw0MTcsNzIwLDgzXV0sIndpZHRoIjo3MjAsImhlaWdodCI6NTAwfQ==";

    // 執行解析
    let result = decode_jm_canvas_data(raw_base64).expect("解碼與解析應該要成功");

    // 1. 驗證寬高是否正確 (從你給的 HTML 可知是 720x500)
    assert_eq!(result.width, 720.0);
    assert_eq!(result.height, 500.0);

    // 2. 驗證切割區塊數量 (這張圖被切成了 6 塊)
    assert_eq!(result.args.len(), 6);

    // 3. 驗證第一塊的座標是否完全符合 [0, 415, 720, 85, 0, 0, 720, 85]
    let first_arg = result.args[0];
    assert_eq!(first_arg, [0.0, 415.0, 720.0, 85.0, 0.0, 0.0, 720.0, 85.0]);

    // 4. 驗證最後一塊的座標 [0, 0, 720, 83, 0, 417, 720, 83]
    let last_arg = result.args[5];
    assert_eq!(last_arg, [0.0, 0.0, 720.0, 83.0, 0.0, 417.0, 720.0, 83.0]);
}

#[aidoku_test]
fn test_invalid_base64() {
    // 驗證給予錯誤資料時不會崩潰，而是回傳 None
    let result = decode_jm_canvas_data("這不是正確的Base64");
    assert_eq!(result, None);
}