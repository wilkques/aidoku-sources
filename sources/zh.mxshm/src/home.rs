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

use crate::{Mxshm, fetch::Fetch, html::GenManga, url::Url};

impl Home for Mxshm {
    fn get_home(&self) -> Result<HomeLayout> {
        send_partial_result(&HomePartialResult::Layout(HomeLayout {
            components: vec![
                HomeComponent {
                    title: Some("今日更新".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_big_scroller(),
                },
                HomeComponent {
                    title: Some("人氣榜".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
                HomeComponent {
                    title: Some("推薦榜".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
                HomeComponent {
                    title: Some("完結榜".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
            ],
        }));

        let request = Fetch::get(Url::rank(1)?.to_string())?.html()?;

        let binding = GenManga::list(&request)?;

        let mut chunks = binding.entries.chunks(10).map(|chunk| chunk.to_vec());

        let dailymanga = chunks.next().unwrap_or_default();
        let popularitymanga = chunks.next().unwrap_or_default();
        let recommendmanga = chunks.next().unwrap_or_default();
        let finishmanga = chunks.next().unwrap_or_default();

        let mut components = Vec::new();

        if !dailymanga.is_empty() {
            components.push(HomeComponent {
                title: Some("今日更新".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: true,
                    page_size: Some(3),
                    entries: dailymanga.into_iter().map(|manga| manga.into()).collect(),
                    listing: Some(Listing {
                        id: "dailymanga".to_string(),
                        name: "今日更新".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        if !popularitymanga.is_empty() {
            components.push(HomeComponent {
                title: Some("人氣榜".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: true,
                    page_size: Some(3),
                    entries: popularitymanga
                        .into_iter()
                        .map(|manga| manga.into())
                        .collect(),
                    listing: Some(Listing {
                        id: "popularitymanga".to_string(),
                        name: "人氣榜".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        if !recommendmanga.is_empty() {
            components.push(HomeComponent {
                title: Some("推薦榜".to_string()),
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
                        name: "推薦榜".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        if !finishmanga.is_empty() {
            components.push(HomeComponent {
                title: Some("完結榜".to_string()),
                subtitle: None,
                value: HomeComponentValue::MangaList {
                    ranking: true,
                    page_size: Some(3),
                    entries: finishmanga.into_iter().map(|manga| manga.into()).collect(),
                    listing: Some(Listing {
                        id: "finishmanga".to_string(),
                        name: "完結榜".to_string(),
                        kind: ListingKind::Default,
                    }),
                },
            });
        }

        Ok(HomeLayout { components })
    }
}
