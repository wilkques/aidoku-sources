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

        let results: [Result<Vec<Manga>>; 6] = responses
            .map(|res| res?.get_html()?.list())
            .map(|res| Ok(res?.entries));

        let [popularity, weekly, korean, mainland, japanese, ongoing] = results;
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
