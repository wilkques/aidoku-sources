use aidoku::{
    Home, HomeComponent, HomeComponentValue, HomeLayout, HomePartialResult, Listing, ListingKind,
    Manga, Result,
    alloc::{String, Vec, string::ToString as _, vec},
    error,
    imports::{
        net::{Request, RequestError, Response},
        std::send_partial_result,
    },
};

use crate::{Dm5, fetch::Fetch, html::GenManga, url::Url};

impl Home for Dm5 {
    fn get_home(&self) -> Result<HomeLayout> {
        send_partial_result(&HomePartialResult::Layout(HomeLayout {
            components: vec![
                HomeComponent {
                    title: Some("今日更新".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_big_scroller(),
                },
                HomeComponent {
                    title: Some("日漫排行".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
                HomeComponent {
                    title: Some("陸漫排行".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
                HomeComponent {
                    title: Some("綜合排行".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
                HomeComponent {
                    title: Some("上升最快".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
            ],
        }));

        let responses: [core::result::Result<Response, RequestError>; 5] = Request::send_all([
            Fetch::get(Url::rank("new".to_string())?.to_string())?,
            Fetch::get(Url::rank("2".to_string())?.to_string())?,
            Fetch::get(Url::rank("1".to_string())?.to_string())?,
            Fetch::get(Url::rank("3".to_string())?.to_string())?,
            Fetch::get(Url::rank("7".to_string())?.to_string())?,
        ])
        .try_into()
        .map_err(|_| error!("Failed to convert requests vec to array"))?;

        let results: [Result<Vec<Manga>>; 5] = responses
            .map(|res| res?.get_html()?.list())
            .map(|res| Ok(res?.entries));

        let [daily_rank, jpmanga_rank, cnmanga_rank, all_rank, rise_rank] = results;
        let dailymanga = daily_rank?;
        let jpmanga = jpmanga_rank?;
        let cnmanga = cnmanga_rank?;
        let allmanga = all_rank?;
        let risemanga = rise_rank?;

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

        if !jpmanga.is_empty() {
            components.push(HomeComponent {
                title: Some("日漫排行".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: true,
                    page_size: Some(3),
                    entries: jpmanga.into_iter().map(|manga| manga.into()).collect(),
                    listing: Some(Listing {
                        id: "jpmanga".to_string(),
                        name: "日漫排行".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        if !cnmanga.is_empty() {
            components.push(HomeComponent {
                title: Some("陸漫排行".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: true,
                    page_size: Some(3),
                    entries: cnmanga.into_iter().map(|manga| manga.into()).collect(),
                    listing: Some(Listing {
                        id: "cnmanga".to_string(),
                        name: "陸漫排行".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        if !allmanga.is_empty() {
            components.push(HomeComponent {
                title: Some("綜合排行".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: true,
                    page_size: Some(3),
                    entries: allmanga.into_iter().map(|manga| manga.into()).collect(),
                    listing: Some(Listing {
                        id: "allmanga".to_string(),
                        name: "綜合排行".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        if !risemanga.is_empty() {
            components.push(HomeComponent {
                title: Some("上升最快".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: true,
                    page_size: Some(3),
                    entries: risemanga.into_iter().map(|manga| manga.into()).collect(),
                    listing: Some(Listing {
                        id: "risemanga".to_string(),
                        name: "上升最快".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        Ok(HomeLayout { components })
    }
}
