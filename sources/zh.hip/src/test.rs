// #![expect(clippy::unwrap_used)]

// use core::panic;

// use super::*;
// // use aidoku::Home;
// use aidoku::MangaStatus;
// use aidoku::imports::html::Html;
use aidoku_test::aidoku_test;

// #[aidoku_test]
// fn test_list_parsing_offline() {
//     // 離線 fixture，不打網路，用來單獨驗證 GenManga::list 的選擇器邏輯
//     let html = include_str!("../docs/popularity.html");

//     let document = Html::parse_with_url(html, "https://m.hipmh.com/popularity").unwrap();

//     let result = document.list().unwrap();

//     panic!("完整結果: {:#?}", result);

//     assert_eq!(result.entries.len(), 18);
//     assert!(result.has_next_page);

//     let first = &result.entries[0];
//     assert_eq!(first.key, "bToyMzQ3NQ-yi-ren-zhi-xia-tx-531490-17793");
//     assert_eq!(first.title, "一人之下");
//     assert_eq!(
//         first.url.as_deref(),
//         Some("https://m.hipmh.com/works/bToyMzQ3NQ-yi-ren-zhi-xia-tx-531490-17793")
//     );
//     assert!(first.cover.as_deref().unwrap_or_default().starts_with("https://cover.s3imgs.top/"));
// }

// #[aidoku_test]
// fn test_detail_parsing_offline() {
//     // 離線 fixture，不打網路，用來單獨驗證 GenManga::detail 的選擇器邏輯
//     let html = include_str!("../docs/detail.html");

//     let document = Html::parse_with_url(
//         html,
//         "https://m.hipmh.com/works/bToyMzQ3NQ-yi-ren-zhi-xia-tx-531490-17793",
//     )
//     .unwrap();

//     let mut manga = Manga::default();
//     document.detail(&mut manga).unwrap();

//     assert_eq!(manga.title, "一人之下");
//     assert_eq!(
//         manga.authors,
//         Some(vec!["米二".to_string(), "米橙子".to_string()])
//     );
//     assert_eq!(
//         manga.tags,
//         Some(vec!["战斗".to_string(), "搞笑".to_string()])
//     );
//     assert_eq!(manga.status, MangaStatus::Ongoing);
//     assert!(
//         manga
//             .cover
//             .as_deref()
//             .unwrap_or_default()
//             .starts_with("https://cover.s3imgs.top/")
//     );
//     assert!(
//         manga
//             .description
//             .as_deref()
//             .unwrap_or_default()
//             .starts_with("【一人之下")
//     );
// }

// #[aidoku_test]
// fn test_decode_chapter_images_offline() {
//     // 離線 fixture（真實 API 回應），驗證反混淆 chapter-decoder.js 後重寫的解碼邏輯
//     let json = include_str!("../docs/chapter_images.json");
//     let value: serde_json::Value = serde_json::from_str(json).unwrap();
//     let images = value["data"]["images"].as_str().unwrap();

//     let paths = crate::decoder::decode_chapter_images(images).unwrap();

//     assert_eq!(paths.len(), 18);
//     assert_eq!(
//         paths[0],
//         "/i/zOWt2hzKWm3pS8DS0BeY5s1H8cUvKJ-YppXpwNLq6EUAUMG58H2bCSVLK-5CsrW0wisPfB2hOu1v-0eE6S2sdbvLjSf3kaxWTIw/dHg6MjM0NzU6MToxOjMzMjAx_1.36n7im.webp"
//     );
//     assert_eq!(
//         paths[17],
//         "/i/zOWt2hzKWm3pS8DS0BeY5s1H8cUvKJ-YppXpwNLq6EUAUMG58H2bCSVLK-5CsrW0wisPfB2hOu1v-0eE6S2sdbvLjSf3kaxWTIw/dHg6MjM0NzU6MToxNzo3ODAz_17.ktkxse.webp"
//     );
// }

// #[aidoku_test]
// fn test_get_search_manga_list() {
//     let source = Hip::new();

//     let filters = vec![
//         // FilterValue::Select {
//         //     id: String::from("列表"),
//         //     value: String::from("update"),
//         // },
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
//         .get_search_manga_list(None, 2, filters)
//         .unwrap();

//     panic!("完整結果: {:#?}", result);
// }

// #[aidoku_test]
// fn test_get_manga_update() {
//     let source = Mxs::new();

//     // 1. 建立一個假的 Manga 用於測試
//     let manga = Manga {
//         key: "1148".to_string(), // 換成真實的漫畫 ID 以測試
//         title: "華爾街夜色".to_string(),
//         cover: Some("https://www.jjmhw2.top/static/upload/book/1148/cover.jpg".to_string()),
//         url: Some(
//             "https://www.mxs13.cc/book/1148".to_string(),
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
//     let source = Mxs::new();

//     let manga = Manga {
//         key: "1142".to_string(), // 換成真實的漫畫 ID 以測試
//         title: "詛咒性轉物語".to_string(),
//         cover: Some("https://www.jjmhw2.top/static/upload/book/1142/cover.jpg".to_string()),
//         ..Default::default()
//     };

//     let chapter = Chapter {
//         key: "52752".to_string(), // 換成真實的漫畫 ID 以測試
//         title: Some("第1話-睡醒變成發春女".to_string()),
//         chapter_number: Some(1.0),
//         ..Default::default()
//     };

//     // 2. 傳入正確的三個參數
//     let result = source
//         .get_page_list(manga, chapter)
//         .unwrap();

//     panic!("完整結果: {:#?}", result);
// }

// #[aidoku_test]
// fn test_handle_deep_link() {
//     let source = Mxs::new();

//     // 測試案例 1: 有效的網址
//     let valid_url = "https://www.mxs13.cc/book/52752".to_string();

//     let result = source.handle_deep_link(valid_url).unwrap();

//     panic!("✅ 成功解析 DeepLink: {:?}", result);
// }

// #[aidoku_test]
// fn test_get_home() {
//     let source = Mxs::new();

//     let result = source.get_home();

//     panic!("✅ 成功解析 DeepLink: {:#?}", result);
// }

#[aidoku_test]
fn test_category_list_parsing_offline() {
    // 離線 fixture（真實 API 回應），驗證 genre/category/status 篩選改走 JSON 解析後邏輯正確
    // （之前誤用 .html() 解析 hipapi1.s3file.top 回的 JSON，導致選韓漫/陸漫/日漫/連載中卡住）
    let json = include_str!("../docs/category.json");

    let result = crate::json::parse_manga_list_json_str(json).unwrap();

    assert!(result.has_next_page);
    assert!(!result.entries.is_empty());

    let first = &result.entries[0];
    assert_eq!(first.key, "bToxNTIyMQ-wei-zhuang-shang-liu-15214");
    assert_eq!(first.title, "伪装上流");
    assert_eq!(
        first.url.as_deref(),
        Some("https://m.hipmh.com/works/bToxNTIyMQ-wei-zhuang-shang-liu-15214")
    );
    assert_eq!(
        first.cover.as_deref(),
        Some(
            "https://cover.s3imgs.top/kk/vertical/wei-zhuang-shang-liu-15214-d2VpLXpodWFuZy1zaGFuZy1saXUtMTUyMTQ.webp"
        )
    );
}

#[aidoku_test]
fn test_search_parsing_offline() {
    // 離線 fixture（真實 API 回應），驗證搜尋改走 hipapi1.s3file.top 的 JSON 解析
    // （之前打 m.hipmh.com/search 又用 .html() 解析，跟 category 篩選是同一類卡住的 bug）
    let json = include_str!("../docs/search.json");

    let result = crate::json::parse_search_json_str(json).unwrap();

    assert!(!result.entries.is_empty());

    let first = &result.entries[0];
    assert_eq!(first.key, "bToyMzQ3NQ-yi-ren-zhi-xia-tencent-531490-17793");
    assert_eq!(first.title, "一人之下");
    assert_eq!(
        first.url.as_deref(),
        Some("https://m.hipmh.com/works/bToyMzQ3NQ-yi-ren-zhi-xia-tencent-531490-17793")
    );
}
