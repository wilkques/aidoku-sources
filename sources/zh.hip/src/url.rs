use aidoku::{
    FilterValue, Result,
    alloc::{String, string::ToString as _},
    helpers::uri::encode_uri,
    prelude::*,
};

use crate::settings;

// filters.json 的「题材」select 沒有 ids，value 會是選項的中文字面值，
// 所以用 (label, 對應值) 對照表直接查，而不是用 index 去猜落在哪個區間。
const PATH_OPTIONS: &[(&str, &str)] = &[
    ("人氣榜", "popularity"),
    ("本周熱門", "weekly"),
];

const GENRE_OPTIONS: &[(&str, i32)] = &[
    ("系統", 67),
    ("玄幻", 27),
    ("穿越", 20),
    ("大女主", 30),
    ("逆襲", 32),
    ("武俠", 39),
    ("重生", 46),
    ("動作", 40),
    ("冒險", 38),
    ("復仇", 31),
];

// 語系 1 韓漫 2 陸漫 3 日漫
const CATEGORY_OPTIONS: &[(&str, i32)] = &[
    ("韓漫", 1),
    ("陸漫", 2),
    ("日漫", 3),
];

const STATUS_OPTIONS: &[(&str, &str)] = &[
    ("連載中", "ongoing"),
    ("已完結", "completed"),
];

#[derive(Clone)]
pub enum FilterKind {
    Path(String),
    Genre(i32),
    Category(i32),
    Status(String),
}

impl FilterKind {
    /// Genre/Category/Status 是打 hipapi1.s3file.top 拿 JSON，不是 m.hipmh.com 的 HTML 頁
    pub fn is_json_api(&self) -> bool {
        matches!(
            self,
            FilterKind::Genre(_) | FilterKind::Category(_) | FilterKind::Status(_)
        )
    }
}

#[derive(Clone)]
pub enum Url {
    Filter { kind: FilterKind, page: i32 },
    Search { query: String, page: i32 },
    Chapter { id: String },
    Book { id: String },
}

impl Url {
    pub fn to_string(&self) -> String {
        let page_size = 20;
        let base_url = settings::get_base_url();

        match self {
            Self::Chapter { id } => {
                format!("{}/chapter/{}", settings::get_reader_url(), id)
            }
            Self::Book { id } => {
                format!("{}/works/{}", base_url, id)
            }
            Self::Search { query, page } => {
                // 搜尋也是打 hipapi1.s3file.top 拿 JSON，不是 m.hipmh.com 的 HTML 頁
                format!(
                    "{}/v1/search?q={}&page={}&page_size={}",
                    settings::get_api_url(),
                    query,
                    page,
                    page_size
                )
            }
            Self::Filter { kind, page } => {
                let get_api_url = settings::get_api_url();

                match kind {
                    FilterKind::Path(p) => format!("{}/{}?page={}", base_url, p, page),
                    FilterKind::Genre(g) => format!(
                        "{}/v1/mangas?genre={}&sort=updated&page={}&page_size={}",
                        get_api_url, g, page, page_size
                    ),
                    FilterKind::Category(c) => format!(
                        "{}/v1/mangas?category={}&sort=updated&page={}&page_size={}",
                        get_api_url, c, page, page_size
                    ),
                    FilterKind::Status(s) => format!(
                        "{}/v1/mangas?status={}&sort=updated&page={}&page_size={}",
                        get_api_url, s, page, page_size
                    ),
                }
            }
        }
    }

    pub fn filters(query: Option<&str>, page: i32, filters: &[FilterValue]) -> Result<Self> {
        if let Some(q) = query {
            return Ok(Self::Search {
                query: encode_uri(q),
                page,
            });
        }

        for filter in filters {
            // 點擊作者（supportsAuthorSearch）傳的是 FilterValue::Text，
            // id 固定是 "author"（參考 zh.jmtt 的 "作者"/"author" 處理）
            if let FilterValue::Text { id, value } = filter {
                if id == "author" {
                    return Ok(Self::Search {
                        query: encode_uri(value.clone()),
                        page,
                    });
                }

                continue;
            }

            let FilterValue::Select { id, value } = filter else {
                continue;
            };

            // 點漫畫上的標籤（genre）傳的是 Select，不是 Text（參考 zh.jmtt）。
            // manga.tags 只有顯示文字、沒有對應的 slug，直接當文字搜尋用——
            // /v1/search 本身就吃得到類型關鍵字，不用另外做 slug 對照表。
            if id == "genre" {
                return Ok(Self::Search {
                    query: encode_uri(value.clone()),
                    page,
                });
            }

            if id != "題材" {
                continue;
            }

            let v = value.as_str();

            if let Some((_, p)) = PATH_OPTIONS.iter().find(|(label, _)| *label == v) {
                return Ok(Self::Filter {
                    kind: FilterKind::Path(p.to_string()),
                    page,
                });
            }

            if let Some((_, g)) = GENRE_OPTIONS.iter().find(|(label, _)| *label == v) {
                return Ok(Self::Filter {
                    kind: FilterKind::Genre(*g),
                    page,
                });
            }

            if let Some((_, c)) = CATEGORY_OPTIONS.iter().find(|(label, _)| *label == v) {
                return Ok(Self::Filter {
                    kind: FilterKind::Category(*c),
                    page,
                });
            }

            if let Some((_, s)) = STATUS_OPTIONS.iter().find(|(label, _)| *label == v) {
                return Ok(Self::Filter {
                    kind: FilterKind::Status(s.to_string()),
                    page,
                });
            }
        }

        Ok(Self::Filter {
            kind: FilterKind::Path("popularity".to_string()),
            page,
        })
    }

    pub fn book(id: String) -> Result<Self> {
        Ok(Self::Book { id })
    }

    pub fn chapter(id: String) -> Result<Self> {
        Ok(Self::Chapter { id })
    }
}
