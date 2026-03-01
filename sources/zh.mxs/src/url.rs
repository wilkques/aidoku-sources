use aidoku::{
    FilterValue, Result,
    alloc::{String, string::ToString as _},
    helpers::uri::encode_uri,
    prelude::*,
};

use crate::settings;

#[derive(Clone)]
pub enum Url {
    Filter {
        tag: String,
        area: String,
        end: String,
        page: i32,
    },
    Search {
        query: String,
        page: i32,
    },
    Chapter {
        id: String,
    },
    ListType {
        list_type: String,
        page: i32,
    },
    Book {
        id: String,
    },
}

impl Url {
    pub fn to_string(&self) -> String {
        let base_url = settings::get_base_url();

        match self {
            Self::Chapter { id } => {
                format!("{}/chapter/{}", base_url, id)
            }
            Self::Book { id } => {
                format!("{}/book/{}", base_url, id)
            }
            Self::Search { query, page } => {
                format!("{}/search?keyword={}&page={}", base_url, query, page)
            }
            Self::ListType { list_type, page } => {
                format!("{}/{}?page={}", base_url, list_type, page)
            }
            Self::Filter {
                tag,
                area,
                end,
                page,
            } => {
                format!(
                    "{}/booklist?tag={}&area={}&end={}&page={}",
                    base_url, tag, area, end, page
                )
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

        let mut tag = String::from("全部");
        let mut area = String::from("-1");
        let mut end = String::from("-1");

        for filter in filters {
            match filter {
                FilterValue::Text { value, .. } => {
                    return Ok(Self::Search {
                        query: encode_uri(value.clone()),
                        page,
                    });
                }
                FilterValue::Sort { id, index, .. } => match id.as_str() {
                    "列表" => {
                        let sort = match index {
                            1 => String::from("update"),
                            _ => String::from("booklist"),
                        };

                        return Ok(Self::ListType {
                            list_type: sort,
                            page,
                        });
                    }
                    _ => continue,
                },
                FilterValue::Select { id, value } => match id.as_str() {
                    "题材" => tag = value.clone(),
                    "地区" => area = value.clone(),
                    "进度" => end = value.clone(),
                    "genre" => tag = value.clone(),
                    _ => continue,
                },
                _ => continue,
            }
        }

        if tag.is_empty() && area.is_empty() && end.is_empty() {
            return Ok(Self::ListType {
                list_type: "booklist".to_string(),
                page,
            });
        }

        Ok(Self::Filter {
            tag,
            area,
            end,
            page,
        })
    }

    pub fn book(id: String) -> Result<Self> {
        Ok(Self::Book { id })
    }

    pub fn chapter(id: String) -> Result<Self> {
        Ok(Self::Chapter { id })
    }

    pub fn rank(page: i32) -> Result<Self> {
        Ok(Self::ListType {
            list_type: "rank".to_string(),
            page,
        })
    }
}
