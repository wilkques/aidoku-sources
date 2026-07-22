#![expect(clippy::unwrap_used)]

use super::*;
// use aidoku::{HashMap, PageContext};
use aidoku_test::aidoku_test;

#[aidoku_test]
fn test_raw_search_api() {
    use crate::{fetch::Fetch, settings};

    let base = settings::get_base_url();
    let url = format!("{}/v2.0/apis/manga/ssearch", base);
    // 2026-07-22 從瀏覽器 DevTools 抓包確認:真實請求是 form-urlencoded,不是 JSON
    // (searchkey=進擊 的 UTF-8 percent-encode)。
    let body = "searchkey=%E9%80%B2%E6%93%8A&v=v2.13&s=web&d=";

    let raw: aidoku::alloc::String = Fetch::post(url)
        .unwrap()
        .header("Content-Type", "application/x-www-form-urlencoded")
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
    use crate::{fetch::Fetch, settings};

    let manga_key = "weilaidegudongdian";
    let chapter_key = "6611564";
    let base = settings::get_base_url();

    let url = format!(
        "{}/v2.0/apis/manga/reading?code={}&cid={}&v=v4.300102",
        base, manga_key, chapter_key
    );

    let raw: aidoku::alloc::String = Fetch::get(url)
        .unwrap()
        .header("Referer", &format!("{}/mangaread/{}/{}", base, manga_key, chapter_key))
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Accept", "application/json")
        .string()
        .unwrap();

    // Print first 2000 chars so we can see ALL API fields including any ordering fields
    let preview = if raw.len() > 2000 { &raw[..2000] } else { &raw };
    panic!("RAW API (first 2000 chars):\n{}", preview);
}

/// Regression test for the 2026-07-22 SECRET rotation
/// (`DEV_SCAN_SECRET_2026_change_me` -> `PRO_SCAN_SECRET_20260712_watching_you_DEBUG`).
/// Captured live from a browser reading-API response so this can be verified
/// offline without hitting the Cloudflare-protected site.
#[aidoku_test]
fn test_decrypt_scans_after_secret_rotation() {
    let enc = "f797120ce727819544ed1e60238dbdde5cd051e904f41350ff961ee90307ed5f97e8193be5251d21f175f2709cb54711bb2d50c8a1bfe9de1cad6427f75356c78e7461e14c2cd2b3b6f986IquDP8UByxRcW1Dw12T7ocp6J9heJGYQhGWtXHwuvkGjZJuJzaxtEQbyKk8sX6kN+W/yf9hXqAt/mgdgzBT0O8OiqbZGq/GB/qirzuLn6+ijo0jCeMKciw/1Ok9ccnjEPeDu1d+eyuLdaCuYByDaK8Bgj9YmhtqCvJ8hh7jze8bXp5UZRQ6pZS0iyspRxN/ke4yuWTgtdHbmDoBxB2emlACg5JVgbY8Ry2Ze0ouuVnKnG006ETJLN2XjpHGIe2rrAjfGll91N5Hfk7YxbAp3DRFlfXMGi6h1GhxlElb+IHKVmEE3fYvpFuSp++dMMj1YTWeES4ne3tEbK1ZhyAJ7Gr788kYpBt7HcsUrEf0FeoS4/p8mTFg4+TNfY4EXVqjdd9GEdScmU4AvdF2BVIeWl7CFjfAMOmAAYp75YLm715NMIpokJcKv8ibHWG+3wAhkvvXscv1eN+Aw52KNIj4VKG5ueUq6hNuoEh75zWItCpNdbTZibBhHXCU8HlA4gS+koTefRDi7oGHbsjSzT8NtOeQpaopIxSk/sidxsV+29yKSlpIc4oBT0XN9CgavN5mVTfqjhKuCRYK9TU2pUYtnOIG+YnfKAWSpQR26ftj+4p0/6COFctlX3MoX+O3ahSNGigovdhQQ9b15IKTdKAno8L5wcXWZbOPcL62/EvLYNRgEV9MFOOvd88rVqLsN74Ts/xUIAUN/J5nC0QM7IbARfeJxDlFN0kR0+OuJIwllFZGll9LAOYWBU5WD26ATod6oVyxGc4713dOy7MQdwxxy9JYblvvloCGPq97hC0NsKWgP1wGmPTWACBh+DF2jc+80k+zjoJzuCjyF94+LCMy6plgzEnocSb+fVeCkE8jFmdZBCQ26ADOWOH16iOY0s81mKcEgi/Tg3yXmmATwrT3sAyMBJkY83QksP9fOlPfg4lqFflxs7WuyxD0O5VZbRy/s1vBPfHRwHas5I0UEU20bsokybSpCrVgWnmWHiW73yG9FytDeau8ALvmPg+BvVIRuaTi/UF/qdYkOKGV/IrlqJTyC9ZCkMIBcouOKjhlrUEWwSSJdfLGTtXPlS6JE2rnvn0MwpUsr3Q9gJIEvZdVJ0wUYnSDVuVcxrzHsZ5/puzpc7pxuUY0pQYBC1mGzETZuyWJAwfvgKgkRkE5B7DjtCWuw2dJPRgnmBjGEhcJxKOKfbXAd3LIJbDzhDZWlDD2+77xSR3B/wgs+Ekz6HD2UL/ap0R3c5cdJKdj/xdLRqkp0YMs3woDdf3hV8Otx2JcREYuRc5GUNmsu0honUChaeMBlX34aG2l0K1vCbY0uCPAU7BZEz6RLphdivzE8gXBNrfrJ75xNR91p5cXyglgVSK97h6DjXyLlQ3cZDoR/e88AAolIkLR+MrNTuwWGNdrJtgvyOdQg2eT21KMvlQyFOmRwE7YhQHr5oK7W+prgKSBiJbdc/1lwun9NEwbien06lusAgRnGSR3t2/SxRPnCUdOdbuPTMxc2bfA1cx519imFcqcm+9b3rvrQAB3WUSllHgcAso51LOE0JW+x78rZh6N+NzBKuW8f497tvXCCYZy8EntPSj2tTONY5qQSJVO52j2dsc8wxxoJYFWwJCYTMj1gpa9oS4BH7bEmApwl9feqQC59p7m3dyzM8jOYMonbbn9SFA5lGZKsH8Ps5rwdAIYDP4LyU7DxW5Fb1VPCZwxq12zicKSgsp6RPhs2Oic54u2MyxezVTvZPGTC8SBLH4qcrUYJBDjAnqcOkpImZWKIpnOCWidwl1TWNGKc3Obk95Y9ijVM4/xnbuZCpMOf2sApja7JQ4fhaIyV5CHMMF1o0Jp3eoe8jOfzrA3xl+0SFyGCFqJyuH2Vk9veQNd/9mbgT7IM973EGjQZwPw1A35HhVJxMezD4YOS9lmN+JAQjVlA219suA9DkAPGWsIseRrid3L6wgnNT8KrfDZQN9Ojc7KXUWpQbXT5hQW0YgZRoAxkoQ3EF70ov+KIyTwYg83qzJHowyX4iyvhO0ehngsX0aVfR1Ldmgo/+rqtTHIa+N19Sa8tix8GdsquBcQ5nSveMtI8s1GYfn6Xr0RO5Q3KJcnakol/m6qXqI/aoYFdlaZJmd4Y7D0r9FJr0Gyb9aImt0zhlfsuCCFNkDWwg60eedGiroPio0oa5X1VTHkRzakuc8NgPmNoW46btcHFaF1mF6e6vMYlYGgUGSfMXHjlF9KNbffAQoUK5bxrTOVOiAXZmathJDYiF7CwOsJ+vY0u/saqVsKh4de6ne3AkqIEk+QH71G3GQAz8T3XB7AsXf6Z1xV1hdsrnHw23Dy5bsDUnlvjWa2nl9emggVNHc9ViB2k6151ZlpZ8QVPedUEtZPinm7Mx1CLLJJfUIdx2ua6arxJTVSkLpRQiCtgaHHDwOZ86eF//glHKahFC9hditu2BhxGmRQf3IQSm4PV96cFf9YlPNDNjPa8ZJxZvtGpTlgXenI7rVtSz5lIXuT0i75pQgg+Me66uVCxvIYR3ZMu+68V8IW4YcEVzLo3dkQ6+B0rF60RY6/Ukxo46qcSbCxGuYPg1oC5X80sqCyEbg80qQDrz8ASp4MUHnu5HllxQ47flMM3cPgNnixplua24wYn0jpqB42zwJZtoUjLfzl+gR9Mjl9vfQ2Mfwu1iTDGeVs/Ld3BDBWY1+1/GvJsb3id/9iWHPkNWQQ4ABqJxXCiNlt5SMaNHolRCXjHXqfbe/8X3GWIkXXOjvabNFCH5u9QFu6cvWx+6KFyrgM8dhYRNkbdF+BndiWK4pLwXmWpMj3TwtNuhF8LyzPM6K6j9XyboOsy8A21PMwL0sRLOWwdbaETD0Umo+m/yjh22VXzJtOj5dPxgspdKdIP6StVCuNIiY7HRenPGfU7QF1y5u8W4pczUwz0o1IT4O3N6fzX4KXORF8HiEAjupcQwTDii0wer5f82nHTTp9NYAYYFwyEFcUS1XLHuvS9bM689v2Rdridf5Vo5r3YiYr1TRHBxsZz+U3nQds48jiO/MxJHAyFL++tbs2DtBIQzT0iWFjxJQLU6Tm1IfaVdKJrKT0XFBlo0zP3Nr6IoqAuTASeD6ECAMk0UxSKKUNXvkI+hR+1C2HW8i/43Dd8BGG4HS94c9EGGnqPJRmvbg1ahodQDDpmG3DHdfMo/CiScPE6COYIIoYahkkQf3he9cVxf8dfJY9c1BPJ8B6tlZnWYajx/Y8uWJtU9IN+nBxL1eoPsaXPEaim100ip3Xn2X14Rv3IXrfCOToEm1yVOWflUpTv4vjT0huH4nCCSlr53XSOBwm3MklfSqCvMa+lWAKHUL8pHLyN7qqv4AicRslnTGIItNZXFFRubtneBn8NSLe/OOueHSBYTh29pipfLFonmPGoFWZCLVdB0CKy3BNzlGsvT9aWu1pc7LXcIKNcHxsbX8gkev1U1DdT+I7spzgi6bycRcBPWBOECXEyNh6iMSwmg00+vWtAAHaNw3FKapjm6s+GZcN+epG1jmM1cNvO6+Z+ABL5ySu+EDlULfPlTaKGzkIPxgURzow0b6U/6gaLlhSEe6ylHdyylM0KBv8z7yn";

    let items = crate::crypto::decrypt_scans(enc, "happymh.com");
    assert!(items.is_some(), "decrypt_scans failed with current SECRET");
    let items = items.unwrap();
    assert!(!items.is_empty());
    assert!(items[0].url.starts_with("https://ruicdn.happymh.com/"));
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
