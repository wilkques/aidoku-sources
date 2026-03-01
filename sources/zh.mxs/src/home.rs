use aidoku::{
    Home, HomeComponent, HomeComponentValue, HomeLayout, HomePartialResult, Listing, ListingKind,
    Result,
    alloc::{Vec, string::ToString as _, vec},
    imports::std::send_partial_result,
};

use crate::{Mxs, fetch::Fetch, html::GenManga, url::Url};

impl Home for Mxs {
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
                    value: HomeComponentValue::empty_big_scroller(),
                },
                HomeComponent {
                    title: Some("推薦榜".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_big_scroller(),
                },
                HomeComponent {
                    title: Some("完結榜".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::empty_manga_list(),
                },
            ],
        }));

        let request = Fetch::get(Url::rank(1)?.to_string())?.html()?;

        let binding = GenManga::home(&request)?;

        let mut chunks = binding.into_iter();

        let dailymanga = chunks.next().unwrap_or_default();
        let popularitymanga = chunks.next().unwrap_or_default();
        let finishmanga = chunks.next().unwrap_or_default();
        let recommendmanga = chunks.next().unwrap_or_default();

        let mut components = Vec::new();

        if !dailymanga.is_empty() {
            components.push(HomeComponent {
                title: Some("今日更新".to_string()),
                subtitle: None,
                value: HomeComponentValue::BigScroller {
                    entries: dailymanga.into_iter().map(|manga| manga.into()).collect(),
                    auto_scroll_interval: Some(8.0),
                },
            });
        }

        if !popularitymanga.is_empty() {
            components.push(HomeComponent {
                title: Some("人氣榜".to_string()),
                subtitle: None,
                value: HomeComponentValue::BigScroller {
                    entries: popularitymanga
                        .into_iter()
                        .map(|manga| manga.into())
                        .collect(),
                    auto_scroll_interval: Some(8.0),
                },
            });
        }

        if !recommendmanga.is_empty() {
            components.push(HomeComponent {
                title: Some("推薦榜".to_string()),
                subtitle: None,
                value: HomeComponentValue::BigScroller {
                    entries: recommendmanga
                        .into_iter()
                        .map(|manga| manga.into())
                        .collect(),
                    auto_scroll_interval: Some(8.0),
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
