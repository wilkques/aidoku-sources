#![expect(clippy::unwrap_used)]

use super::*;
// use aidoku::{HashMap, PageContext};
use aidoku_test::aidoku_test;

#[aidoku_test]
fn test_raw_search_api() {
    use crate::{fetch::Fetch, settings};

    let base = settings::get_base_url();
    let url = format!("{}/v2.0/apis/manga/ssearch", base);
    let body = "{\"searchkey\":\"進擊\",\"v\":\"v2.13\",\"s\":\"web\",\"d\":\"\",\"page\":1}";

    let raw: aidoku::alloc::String = Fetch::post(url)
        .unwrap()
        .header("Content-Type", "application/json")
        .header("Referer", &format!("{}/sssearch", base))
        .header("X-Requested-With", "XMLHttpRequest")
        .body(body.as_bytes())
        .string()
        .unwrap();

    let preview = if raw.len() > 2000 { &raw[..2000] } else { &raw };
    panic!("RAW SEARCH (first 2000 chars):\n{}", preview);
}

#[aidoku_test]
fn test_get_search_manga_list() {
    let source = Happy::new();

    let filters = vec![
        // FilterValue::Select {
        //     id: String::from("排序"),
        //     value: String::from("s2"),
        // },
        // FilterValue::Select {
        //     id: String::from("題材"),
        //     value: String::from("rexue"),
        // },
        // FilterValue::Select {
        //     id: String::from("地区"),
        //     value: String::from("2"),
        // },
        // FilterValue::Select {
        //     id: String::from("进度"),
        //     value: String::from("0"), // 全部
        // },
    ];

    let result = source
        // .get_search_manga_list(Some("富家女".to_string()), 1, Vec::new())
        // .get_search_manga_list(None, 1, Vec::new())
        .get_search_manga_list(None, 1, filters)
        .unwrap();

    panic!("完整結果: {:#?}", result);
}

// #[aidoku_test]
// fn test_get_manga_update() {
//     let source = Happy::new();

//     // 1. 建立一個假的 Manga 用於測試
//     let manga = Manga {
//         key: "manhua-zaidixiachengchadianbeixinrendehuobanshadiao-quekaoenhui-moxianzhuaidan-huodele-lv-9999-dehuobanmen-".to_string(), // 換成真實的漫畫 ID 以測試
//         title: "在地下城差點被信任的伙伴殺掉，卻靠恩惠「無限轉蛋」獲得了Lv9999的伙伴們，於是向前隊友和世界復仇&對他們說「死好」!".to_string(),
//         cover: Some("https://mhfm9tel.cdndm5.com/75/74167/20211218103817_180x240_32.jpg".to_string()),
//         url: Some("https://www.dm5.cn/book/manhua-zaidixiachengchadianbeixinrendehuobanshadiao-quekaoenhui-moxianzhuaidan-huodele-lv-9999-dehuobanmen-".to_string()),
//         ..Default::default()
//     };

//     // 2. 傳入正確的三個參數
//     let result = source.get_manga_update(manga, true, true).unwrap();

//     panic!("完整結果: {:#?}", result);
// }

#[aidoku_test]
fn test_get_page_list() {
    let source = Happy::new();

    let manga = Manga {
        key: "weilaidegudongdian".to_string(),
        title: "未来的股东店".to_string(),
        ..Default::default()
    };

    let chapter = Chapter {
        key: "6611564".to_string(),
        ..Default::default()
    };

    let result = source.get_page_list(manga, chapter).unwrap();

    panic!("完整結果: {:#?}", result);
}

#[aidoku_test]
fn test_raw_reading_api() {
    use crate::{fetch::Fetch, settings, url::Url};

    let manga_key = "weilaidegudongdian";
    let chapter_key = "6611564";

    let url = Url::chapter(manga_key.to_string(), chapter_key.to_string())
        .unwrap()
        .to_string();

    let raw: aidoku::alloc::String = Fetch::get(url)
        .unwrap()
        .header("Referer", &format!("{}/mangaread/{}/{}", settings::get_base_url(), manga_key, chapter_key))
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Accept", "application/json")
        .string()
        .unwrap();

    // Print first 2000 chars so we can see ALL API fields including any ordering fields
    let preview = if raw.len() > 2000 { &raw[..2000] } else { &raw };
    panic!("RAW API (first 2000 chars):\n{}", preview);
}

// #[aidoku_test]
// fn test_handle_deep_link() {
//     let source = Happy::new();

//     // 測試案例 1: 有效的網址
//     let valid_url = "https://www.mxs13.cc/book/52752".to_string();

//     let result = source.handle_deep_link(valid_url).unwrap();

//     panic!("✅ 成功解析 DeepLink: {:?}", result);
// }

// #[aidoku_test]
// fn test_get_image_request() {
//     let source = Happy::new();

//     // 方法 1: 模擬有 is_chapter context 的場景（從 chapter 頁面擷取圖片）
//     let mut ctx: PageContext = HashMap::new();
//     ctx.insert("is_chapter".to_string(), "true".to_string());

//     // 使用一個真實的章節頁面 URL
//     let url = Url::chapter("m1217932-p2".to_string()).unwrap().to_string();

//     let result = source.get_image_request(url, Some(ctx));

//     panic!("is_chapter=true 結果: {:#?}", result);
// }
