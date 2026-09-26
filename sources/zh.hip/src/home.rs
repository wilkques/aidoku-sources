use aidoku::{
    Home, HomeComponent, HomeComponentValue, HomeLayout, HomePartialResult, Listing, ListingKind,
    Manga, Result,
    alloc::{Vec, string::ToString as _, vec},
    imports::std::send_partial_result,
    prelude::*,
};

use crate::{
    Hip,
    fetch::{Api, Fetch},
    html::GenManga,
    json,
    url::{FilterKind, Url},
};

impl Home for Hip {
    fn get_home(&self) -> Result<HomeLayout> {
        send_partial_result(&HomePartialResult::Layout(HomeLayout {
            components: vec![
                HomeComponent {
                    title: Some("人氣榜".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
                HomeComponent {
                    title: Some("本周熱門".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
                HomeComponent {
                    title: Some("韓漫".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
                HomeComponent {
                    title: Some("陸漫".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
                HomeComponent {
                    title: Some("日漫".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
                HomeComponent {
                    title: Some("連載中".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
            ],
        }));

        // popularity/weekly 是 m.hipmh.com 的 HTML 頁（CDN 快取，直接請求即可）；
        // 韓漫/陸漫/日漫/連載中是 hipapi1.s3file.top 的 JSON，要走 WebView（見 fetch.rs 的 `Api`）
        let html_list = |path: &str| -> Result<Vec<Manga>> {
            let url = Url::Filter {
                kind: FilterKind::Path(path.to_string()),
                page: 1,
            }
            .to_string();

            Ok(Fetch::html(url)?.list()?.entries)
        };

        let popularity = html_list("popularity");
        let weekly = html_list("weekly");

        // 四個 JSON 分類共用一個 WebView；建不起來的話四個都算失敗
        let api = Api::new();
        let json_list = |kind: FilterKind| -> Result<Vec<Manga>> {
            let Ok(api) = &api else {
                bail!("WebView 建立失敗");
            };
            let url = Url::Filter { kind, page: 1 }.to_string();

            Ok(json::fetch_manga_list_json_with(api, &url)?.entries)
        };

        let korean = json_list(FilterKind::Category(1));
        let mainland = json_list(FilterKind::Category(2));
        let japanese = json_list(FilterKind::Category(3));
        let ongoing = json_list(FilterKind::Status("ongoing".to_string()));

        // 六個分類裡有四個打的是 CF 後面的 hipapi1，驗證失效時會被擋（見 docs/RESEARCH.md 第十節）。
        // 個別分類失敗只讓該分類消失就好，不要讓整個首頁跟著掛掉；但六個全滅時要把錯誤傳出去，
        // 否則使用者只會看到一片空白的首頁，不知道發生什麼事。
        let mut lists: [Vec<Manga>; 6] = Default::default();
        let mut first_error = None;

        let results = [popularity, weekly, korean, mainland, japanese, ongoing];

        for (slot, result) in lists.iter_mut().zip(results) {
            match result {
                Ok(entries) => *slot = entries,
                Err(error) => first_error = first_error.or(Some(error)),
            }
        }

        if lists.iter().all(|list| list.is_empty())
            && let Some(error) = first_error
        {
            return Err(error);
        }

        let [popularity, weekly, korean, mainland, japanese, ongoing] = lists;

        let mut components = Vec::new();

        if !popularity.is_empty() {
            components.push(HomeComponent {
                title: Some("人氣榜".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: true,
                    page_size: Some(3),
                    entries: popularity.into_iter().map(|manga| manga.into()).collect(),
                    listing: Some(Listing {
                        id: "popularity".to_string(),
                        name: "人氣榜".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        if !weekly.is_empty() {
            components.push(HomeComponent {
                title: Some("本周熱門".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: true,
                    page_size: Some(3),
                    entries: weekly.into_iter().map(|manga| manga.into()).collect(),
                    listing: Some(Listing {
                        id: "weekly".to_string(),
                        name: "本周熱門".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        if !korean.is_empty() {
            components.push(HomeComponent {
                title: Some("韓漫".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: false,
                    page_size: Some(3),
                    entries: korean.into_iter().map(|manga| manga.into()).collect(),
                    listing: Some(Listing {
                        id: "korean".to_string(),
                        name: "韓漫".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        if !mainland.is_empty() {
            components.push(HomeComponent {
                title: Some("陸漫".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: false,
                    page_size: Some(3),
                    entries: mainland.into_iter().map(|manga| manga.into()).collect(),
                    listing: Some(Listing {
                        id: "mainland".to_string(),
                        name: "陸漫".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        if !japanese.is_empty() {
            components.push(HomeComponent {
                title: Some("日漫".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: false,
                    page_size: Some(3),
                    entries: japanese.into_iter().map(|manga| manga.into()).collect(),
                    listing: Some(Listing {
                        id: "japanese".to_string(),
                        name: "日漫".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        if !ongoing.is_empty() {
            components.push(HomeComponent {
                title: Some("連載中".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: false,
                    page_size: Some(3),
                    entries: ongoing.into_iter().map(|manga| manga.into()).collect(),
                    listing: Some(Listing {
                        id: "ongoing".to_string(),
                        name: "連載中".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        Ok(HomeLayout { components })
    }
}
