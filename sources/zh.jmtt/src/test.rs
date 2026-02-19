#![expect(clippy::unwrap_used)]

// use super::*;
// use aidoku_test::aidoku_test;

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