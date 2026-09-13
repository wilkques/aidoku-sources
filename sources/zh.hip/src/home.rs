use aidoku::{
    Home, HomeComponent, HomeComponentValue, HomeLayout, HomePartialResult, Listing, ListingKind,
    Manga, Result,
    alloc::{Vec, string::ToString as _, vec},
    imports::{
        net::{RequestError, Response},
        std::send_partial_result,
    },
};

use crate::{
    Hip,
    fetch::Fetch,
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

        let responses: [core::result::Result<Response, RequestError>; 6] = [
            Fetch::get(
                Url::Filter {
                    kind: FilterKind::Path("popularity".to_string()),
                    page: 1,
                }
                .to_string(),
            )?
            .send(),
            Fetch::get(
                Url::Filter {
                    kind: FilterKind::Path("weekly".to_string()),
                    page: 1,
                }
                .to_string(),
            )?
            .send(),
            Fetch::get(
                Url::Filter {
                    kind: FilterKind::Category(1),
                    page: 1,
                }
                .to_string(),
            )?
            .send(),
            Fetch::get(
                Url::Filter {
                    kind: FilterKind::Category(2),
                    page: 1,
                }
                .to_string(),
            )?
            .send(),
            Fetch::get(
                Url::Filter {
                    kind: FilterKind::Category(3),
                    page: 1,
                }
                .to_string(),
            )?
            .send(),
            Fetch::get(
                Url::Filter {
                    kind: FilterKind::Status("ongoing".to_string()),
                    page: 1,
                }
                .to_string(),
            )?
            .send(),
        ];

        // popularity/weekly 是 m.hipmh.com 的 HTML 頁；韓漫/陸漫/日漫/連載中是
        // hipapi1.s3file.top 回的 JSON，兩種解析方式不一樣，不能套同一個 .map()
        let [r_popularity, r_weekly, r_korean, r_mainland, r_japanese, r_ongoing] = responses;

        let popularity: Result<Vec<Manga>> = (|| Ok(r_popularity?.get_html()?.list()?.entries))();
        let weekly: Result<Vec<Manga>> = (|| Ok(r_weekly?.get_html()?.list()?.entries))();
        let korean: Result<Vec<Manga>> =
            (|| Ok(json::parse_manga_list_json(r_korean?)?.entries))();
        let mainland: Result<Vec<Manga>> =
            (|| Ok(json::parse_manga_list_json(r_mainland?)?.entries))();
        let japanese: Result<Vec<Manga>> =
            (|| Ok(json::parse_manga_list_json(r_japanese?)?.entries))();
        let ongoing: Result<Vec<Manga>> =
            (|| Ok(json::parse_manga_list_json(r_ongoing?)?.entries))();

        let popularity = popularity?;
        let weekly = weekly?;
        let korean = korean?;
        let mainland = mainland?;
        let japanese = japanese?;
        let ongoing = ongoing?;

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
