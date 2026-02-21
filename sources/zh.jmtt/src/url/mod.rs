use aidoku::{
    FilterValue, Result,
    alloc::{String},
    prelude::*,
};

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
                    return format!("{}/search/photos/{}?main_tag={}&search_query={}&t={}&o={}&page={}", base_url, tag, range, query, timing, sort, page);
                } else {
                    format!("{}/albums/{}?t={}&o={}&page={}", base_url, tag, timing, sort, page)
                }
            }
        }
    }

    pub fn filters(query: Option<&str>, page: i32, filters: &[FilterValue]) -> Result<Self> {
        let mut tag = String::from("");
        let mut sort = String::from("mr");
        let mut timing = String::from("a");
        let mut range = String::from("0");

        for filter in filters {
            match filter {
                FilterValue::Select { id, value } => match id.as_str() {
                    "類型" => tag = value.clone(),
                    "排序" => sort = value.clone(),
                    "時間" => timing = value.clone(),
                    "搜索範圍" => range = value.clone(),
                    _ => continue,
                },
                _ => continue,
            }
        }

        Ok(Self::Filter {
            query: query.unwrap_or_default().into(),
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
}
