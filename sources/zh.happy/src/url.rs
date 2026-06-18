use aidoku::{
    FilterValue, Result,
    alloc::String,
    prelude::*,
};

use crate::settings;

#[derive(Clone)]
pub enum Url {
    Filter {
        genre: String,
        audience: String,
        area: String,
        series_status: String,
        page: i32,
    },
    Search {
        query: String,
    },
    Book {
        id: String,
        page: i32,
    },
}

impl Url {
    pub fn to_string(&self) -> String {
        let base_url = settings::get_base_url();

        match self {
            Self::Book { id, page } => {
                format!("{}/v2.0/apis/manga/chapterByPage?code={}&page={}&order=desc", base_url, id, page)
            }
            Self::Search { .. } => {
                format!("{}/v2.0/apis/manga/ssearch", base_url)
            }
            Self::Filter {
                genre,
                audience,
                area,
                series_status,
                page,
            } => {
                format!(
                    "{}/apis/c/index?genre={}&audience={}&area={}&series_status={}&pn={}",
                    base_url, genre, audience, area, series_status, page
                )
            }
        }
    }

    pub fn filters(query: Option<&str>, page: i32, filters: &[FilterValue]) -> Result<Self> {
        if let Some(q) = query {
            return Ok(Self::Search {
                query: String::from(q),
            });
        }

        let mut genre = String::from("");
        let mut audience = String::from("");
        let mut area = String::from("");
        let mut series_status = String::from("");

        for filter in filters {
            match filter {
                FilterValue::Text { value, .. } => {
                    return Ok(Self::Search {
                        query: value.clone(),
                    });
                }
                FilterValue::Select { id, value } => match id.as_str() {
                    "類型" => genre = value.clone(),
                    "受眾" => audience = value.clone(),
                    "地區" => area = value.clone(),
                    "狀態" => series_status = value.clone(),
                    _ => continue,
                },
                _ => continue,
            }
        }

        Ok(Self::Filter {
            genre,
            audience,
            area,
            series_status,
            page,
        })
    }

    pub fn book(id: String, page: i32) -> Self {
        Self::Book { id, page }
    }
}
