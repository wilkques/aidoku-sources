use aidoku::{
    FilterValue, Home, HomeComponent, HomeComponentValue, HomeLayout, HomePartialResult, Listing,
    ListingKind, Manga, Result,
    alloc::{Vec, string::ToString as _, vec},
    imports::std::send_partial_result,
};

use crate::{Bakamh, fetch::Web, url::Url};

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

        // 四個分類共用同一個 WebView，依序抓（見 fetch.rs 的 `Web`）
        let web = Web::new()?;

        let sort_list = |index: i32| -> Result<Vec<Manga>> {
            let url = Url::filters(
                None,
                1,
                &[FilterValue::Sort {
                    id: "排序".to_string(),
                    index,
                    ascending: false,
                }],
            )?
            .to_string();

            Ok(web.list(&url)?.entries)
        };

        // 依序抓、遇錯即停：被擋時第一個分類就會失敗，不用把四個都重試一輪才回報
        let dailymanga = sort_list(1)?;
        let rankmanga = sort_list(2)?;
        let viewmanga = sort_list(3)?;
        let newmanga = sort_list(4)?;

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
