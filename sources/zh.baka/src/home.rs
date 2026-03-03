use aidoku::{
    FilterValue, Home, HomeComponent, HomeComponentValue, HomeLayout, HomePartialResult, Listing,
    ListingKind, Manga, Result,
    alloc::{Vec, string::ToString as _, vec},
    imports::{
        net::{RequestError, Response},
        std::send_partial_result,
    },
};

use crate::{Bakamh, fetch::Fetch, html::GenManga, url::Url};

impl Home for Bakamh {
    fn get_home(&self) -> Result<HomeLayout> {
        send_partial_result(&HomePartialResult::Layout(HomeLayout {
            components: vec![
                HomeComponent {
                    title: Some("今日更新".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
                HomeComponent {
                    title: Some("評分最高".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
                HomeComponent {
                    title: Some("最多瀏覽".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
                HomeComponent {
                    title: Some("最新發布".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
            ],
        }));

        let responses: [core::result::Result<Response, RequestError>; 4] = [
            Fetch::get(
                Url::filters(
                    None,
                    1,
                    &vec![FilterValue::Sort {
                        id: "排序".to_string(),
                        index: 1,
                        ascending: false,
                    }],
                )?
                .to_string(),
            )?
            .send(),
            Fetch::get(
                Url::filters(
                    None,
                    1,
                    &vec![FilterValue::Sort {
                        id: "排序".to_string(),
                        index: 2,
                        ascending: false,
                    }],
                )?
                .to_string(),
            )?
            .send(),
            Fetch::get(
                Url::filters(
                    None,
                    1,
                    &vec![FilterValue::Sort {
                        id: "排序".to_string(),
                        index: 3,
                        ascending: false,
                    }],
                )?
                .to_string(),
            )?
            .send(),
            Fetch::get(
                Url::filters(
                    None,
                    1,
                    &vec![FilterValue::Sort {
                        id: "排序".to_string(),
                        index: 4,
                        ascending: false,
                    }],
                )?
                .to_string(),
            )?
            .send(),
        ];

        let results: [Result<Vec<Manga>>; 4] = responses
            .map(|res| res?.get_html()?.list())
            .map(|res| Ok(res?.entries));

        let [dailymanga, rankmanga, viewmanga, newmanga] = results;
        let dailymanga = dailymanga?;
        let rankmanga = rankmanga?;
        let viewmanga = viewmanga?;
        let newmanga = newmanga?;

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

        if !rankmanga.is_empty() {
            components.push(HomeComponent {
                title: Some("評分最高".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: true,
                    page_size: Some(3),
                    entries: rankmanga.into_iter().map(|manga| manga.into()).collect(),
                    listing: Some(Listing {
                        id: "rankmanga".to_string(),
                        name: "評分最高".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        if !viewmanga.is_empty() {
            components.push(HomeComponent {
                title: Some("最多瀏覽".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: true,
                    page_size: Some(3),
                    entries: viewmanga.into_iter().map(|manga| manga.into()).collect(),
                    listing: Some(Listing {
                        id: "viewmanga".to_string(),
                        name: "最多瀏覽".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        if !newmanga.is_empty() {
            components.push(HomeComponent {
                title: Some("最新發布".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: true,
                    page_size: Some(3),
                    entries: newmanga.into_iter().map(|manga| manga.into()).collect(),
                    listing: Some(Listing {
                        id: "newmanga".to_string(),
                        name: "最新發布".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        Ok(HomeLayout { components })
    }
}
