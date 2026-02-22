#![expect(clippy::unwrap_used)]

use super::*;
use aidoku_test::aidoku_test;

// #[aidoku_test]
// fn test_get_search_manga_list() {
//     let source = Dm5::new();

//     let filters = vec![
//         // FilterValue::Select {
//         //     id: String::from("排序"),
//         //     value: String::from("s2"),
//         // },
//         // FilterValue::Select {
//         //     id: String::from("題材"),
//         //     value: String::from("rexue"),
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

#[aidoku_test]
fn test_get_manga_update() {
    let source = Dm5::new();

    // 1. 建立一個假的 Manga 用於測試
    let manga = Manga {
        key: "manhua-zaidixiachengchadianbeixinrendehuobanshadiao-quekaoenhui-moxianzhuaidan-huodele-lv-9999-dehuobanmen-".to_string(), // 換成真實的漫畫 ID 以測試
        title: "在地下城差點被信任的伙伴殺掉，卻靠恩惠「無限轉蛋」獲得了Lv9999的伙伴們，於是向前隊友和世界復仇&對他們說「死好」!".to_string(),
        cover: Some("https://mhfm9tel.cdndm5.com/75/74167/20211218103817_180x240_32.jpg".to_string()),
        url: Some("https://www.dm5.cn/book/manhua-zaidixiachengchadianbeixinrendehuobanshadiao-quekaoenhui-moxianzhuaidan-huodele-lv-9999-dehuobanmen-".to_string()),
        ..Default::default()
    };

    // 2. 傳入正確的三個參數
    let result = source.get_manga_update(manga, true, true).unwrap();

    panic!("完整結果: {:#?}", result);
}

// #[aidoku_test]
// fn test_get_page_list() {
//     let source = Mxshm::new();

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
//     let source = Mxshm::new();

//     // 測試案例 1: 有效的網址
//     let valid_url = "https://www.mxs13.cc/book/52752".to_string();

//     let result = source.handle_deep_link(valid_url).unwrap();

//     panic!("✅ 成功解析 DeepLink: {:?}", result);
// }
