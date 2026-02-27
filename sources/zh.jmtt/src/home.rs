use aidoku::{
    FilterValue, Home, HomeComponent, HomeComponentValue, HomeLayout, HomePartialResult, Listing,
    ListingKind, Manga, Result,
    alloc::{String, Vec, string::ToString as _, vec},
    error,
    imports::{
        net::{Request, RequestError, Response},
        std::send_partial_result,
    },
};

use crate::{Jmtt, fetch::Fetch, helpers, html::GenManga, url::Url};

impl Home for Jmtt {
    fn get_home(&self) -> Result<HomeLayout> {
        send_partial_result(&HomePartialResult::Layout(HomeLayout {
            components: vec![
                HomeComponent {
                    title: Some("今日更新".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
                HomeComponent {
                    title: Some("最新漫畫".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
                HomeComponent {
                    title: Some("最新韓漫".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
                HomeComponent {
                    title: Some("禁漫漢化組".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
                HomeComponent {
                    title: Some("推薦本本".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
                HomeComponent {
                    title: Some("禁漫去碼&全彩化".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
                HomeComponent {
                    title: Some("單行本".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
                HomeComponent {
                    title: Some("已完結".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
            ],
        }));

        let responses: [core::result::Result<Response, RequestError>; 8] = Request::send_all([
            Fetch::get(
                Url::serialization(helpers::get_current_day_of_week().to_string())?.to_string(),
            )?,
            Fetch::get(
                Url::filters(
                    None,
                    1,
                    &vec![FilterValue::Select {
                        id: "排序".to_string(),
                        value: "mr".to_string(),
                    }],
                )?
                .to_string(),
            )?,
            Fetch::get(
                Url::filters(
                    Some("禁漫汉化组"),
                    1,
                    &vec![FilterValue::Select {
                        id: "搜索範圍".to_string(),
                        value: "0".to_string(),
                    }],
                )?
                .to_string(),
            )?,
            Fetch::get(
                Url::filters(
                    None,
                    1,
                    &vec![FilterValue::Select {
                        id: "類型".to_string(),
                        value: "hanman".to_string(),
                    }],
                )?
                .to_string(),
            )?,
            Fetch::get(Url::promotes("29".to_string(), 1)?.to_string())?,
            Fetch::get(Url::promotes("30".to_string(), 1)?.to_string())?,
            Fetch::get(
                Url::filters(
                    None,
                    1,
                    &vec![FilterValue::Select {
                        id: "類型".to_string(),
                        value: "single".to_string(),
                    }],
                )?
                .to_string(),
            )?,
            Fetch::get(Url::serialization("0".to_string())?.to_string())?,
        ])
        .try_into()
        .map_err(|_| error!("Failed to convert requests vec to array"))?;

        let results: [Result<Vec<Manga>>; 8] = responses
            .map(|res| res?.get_html()?.list())
            .map(|res| Ok(res?.entries));

        let [
            dailymanga,
            newmanga,
            jingmanchinesemanga,
            hanmanga,
            recommendmanga,
            jingmanga,
            offprintmanga,
            finishmanga,
        ] = results;
        let dailymanga = dailymanga?;
        let newmanga = newmanga?;
        let jingmanchinesemanga = jingmanchinesemanga?;
        let hanmanga = hanmanga?;
        let recommendmanga = recommendmanga?;
        let jingmanga = jingmanga?;
        let offprintmanga = offprintmanga?;
        let finishmanga = finishmanga?;

        let mut components = Vec::new();

        if !dailymanga.is_empty() {
            components.push(HomeComponent {
                title: Some("今日更新".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: true,
                    page_size: Some(1),
                    entries: dailymanga.into_iter().map(|manga| manga.into()).collect(),
                    listing: Some(Listing {
                        id: "dailymanga".to_string(),
                        name: "今日更新".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        if !newmanga.is_empty() {
            components.push(HomeComponent {
                title: Some("最新漫畫".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: true,
                    page_size: Some(1),
                    entries: newmanga.into_iter().map(|manga| manga.into()).collect(),
                    listing: Some(Listing {
                        id: "newmanga".to_string(),
                        name: "最新漫畫".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        if !hanmanga.is_empty() {
            components.push(HomeComponent {
                title: Some("最新韓漫".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: true,
                    page_size: Some(1),
                    entries: hanmanga.into_iter().map(|manga| manga.into()).collect(),
                    listing: Some(Listing {
                        id: "hanmanga".to_string(),
                        name: "最新韓漫".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        if !jingmanchinesemanga.is_empty() {
            components.push(HomeComponent {
                title: Some("禁漫漢化組".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: true,
                    page_size: Some(3),
                    entries: jingmanchinesemanga
                        .into_iter()
                        .map(|manga| manga.into())
                        .collect(),
                    listing: Some(Listing {
                        id: "jingmanchinesemanga".to_string(),
                        name: "禁漫漢化組".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        if !recommendmanga.is_empty() {
            components.push(HomeComponent {
                title: Some("推薦本本".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: true,
                    page_size: Some(3),
                    entries: recommendmanga
                        .into_iter()
                        .map(|manga| manga.into())
                        .collect(),
                    listing: Some(Listing {
                        id: "recommendmanga".to_string(),
                        name: "推薦本本".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        if !jingmanga.is_empty() {
            components.push(HomeComponent {
                title: Some("禁漫去碼&全彩化".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: true,
                    page_size: Some(3),
                    entries: jingmanga.into_iter().map(|manga| manga.into()).collect(),
                    listing: Some(Listing {
                        id: "jingmanga".to_string(),
                        name: "禁漫去碼&全彩化".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        if !offprintmanga.is_empty() {
            components.push(HomeComponent {
                title: Some("單行本".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: true,
                    page_size: Some(3),
                    entries: offprintmanga
                        .into_iter()
                        .map(|manga| manga.into())
                        .collect(),
                    listing: Some(Listing {
                        id: "offprintmanga".to_string(),
                        name: "單行本".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        if !finishmanga.is_empty() {
            components.push(HomeComponent {
                title: Some("已完結".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: true,
                    page_size: Some(1),
                    entries: finishmanga.into_iter().map(|manga| manga.into()).collect(),
                    listing: Some(Listing {
                        id: "finishmanga".to_string(),
                        name: "已完結".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        Ok(HomeLayout { components })
    }
}
