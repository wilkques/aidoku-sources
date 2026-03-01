use aidoku::{FilterValue, Result, alloc::String, prelude::*};

use crate::settings;

#[derive(Clone)]
pub enum Url {
    Filter {
        query: String,
        tag: String,
        sort: String,
        timing: String,
        range: String,
        page: i32,
    },
    Chapter {
        id: String,
    },
    Book {
        id: String,
    },
    Serialization {
        id: String,
    },
    Promotes {
        id: String,
        page: i32,
    },
}

impl Url {
    pub fn to_string(&self) -> String {
        let base_url = settings::get_base_url();

        match self {
            Self::Chapter { id } => {
                format!("{}/photo/{}", base_url, id)
            }
            Self::Book { id } => {
                format!("{}/album/{}", base_url, id)
            }
            Self::Filter {
                query,
                tag,
                sort,
                timing,
                range,
                page,
            } => {
                if !query.is_empty() {
                    return format!(
                        "{}/search/photos/{}?main_tag={}&search_query={}&t={}&o={}&page={}",
                        base_url, tag, range, query, timing, sort, page
                    );
                } else {
                    format!(
                        "{}/albums/{}?t={}&o={}&page={}",
                        base_url, tag, timing, sort, page
                    )
                }
            }
            Self::Serialization { id } => {
                format!("{}/serialization/{}", base_url, id)
            }
            Self::Promotes { id, page } => {
                format!("{}/promotes/{}?page={}", base_url, id, page)
            }
        }
    }

    pub fn filters(query: Option<&str>, page: i32, filters: &[FilterValue]) -> Result<Self> {
        let mut query = query.unwrap_or_default();
        let mut tag = String::from("");
        let mut sort = String::from("mr");
        let mut timing = String::from("a");
        let mut range = String::from("0");
        let mut genre: Option<String> = None;
        let mut author: Option<String> = None;

        for filter in filters {
            match filter {
                FilterValue::Text { id, value } => match id.as_str() {
                    "作品" => {
                        query = value;
                        range = String::from("1");
                    }
                    "作者" => author = Some(value.clone()),
                    "標籤" => {
                        query = value;
                        range = String::from("3");
                    }
                    "登場人物" => {
                        query = value;
                        range = String::from("4");
                    }
                    "author" => author = Some(value.clone()),
                    _ => continue,
                },
                FilterValue::Select { id, value } => match id.as_str() {
                    "類型" => tag = value.clone(),
                    "genre" => genre = Some(value.clone()),
                    _ => continue,
                },
                FilterValue::Sort { id, index, .. } => match id.as_str() {
                    "排序" => {
                        sort = match index {
                            1 => String::from("mv"),
                            2 => String::from("mp"),
                            3 => String::from("tr"),
                            4 => String::from("md"),
                            5 => String::from("tl"),
                            _ => String::from("mr"),
                        }
                    }
                    "時間" => {
                        timing = match index {
                            1 => String::from("t"),
                            2 => String::from("w"),
                            3 => String::from("m"),
                            _ => String::from("a"),
                        }
                    }
                    _ => continue,
                },
                _ => continue,
            }
        }

        let (final_query, range) = match (query, genre, author) {
            (q, _, _) if !q.is_empty() => (String::from(q), range),
            (_, Some(g), _) => (g, String::from("3")),
            (_, _, Some(a)) => (a, String::from("2")),
            _ => (String::new(), range),
        };

        Ok(Self::Filter {
            query: final_query,
            tag,
            sort,
            timing,
            range,
            page,
        })
    }

    pub fn book(id: String) -> Result<Self> {
        Ok(Self::Book { id })
    }

    pub fn chapter(id: String) -> Result<Self> {
        Ok(Self::Chapter { id })
    }

    pub fn serialization(id: String) -> Result<Self> {
        Ok(Self::Serialization { id })
    }

    pub fn promotes(id: String, page: i32) -> Result<Self> {
        Ok(Self::Promotes { id, page })
    }
}
