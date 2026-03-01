use aidoku::{
    FilterValue, Result,
    alloc::{String, Vec, string::ToString as _},
    helpers::uri::encode_uri,
    prelude::*,
};

use crate::settings;

#[derive(Clone)]
pub enum Url {
    Filter {
        query: String,
        op: String,
        genre: Vec<String>,
        author: String,
        artist: String,
        release: String,
        adult: String,
        status: Vec<String>,
        m_orderby: String,
        page: i32,
    },
    Search {
        by: String,
        query: String,
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
    fn uri_encode(key: String, value: String, contact: Option<&str>) -> String {
        format!(
            "{}{}{}={}",
            contact.unwrap_or("&"),
            key,
            encode_uri("[]"),
            encode_uri(encode_uri(value)).to_lowercase()
        )
    }

    pub fn to_string(&self) -> String {
        let base_url = settings::get_base_url();

        match self {
            Self::Chapter { id } => {
                format!("{}/manga/{}/", base_url, id)
            }
            Self::Book { id } => {
                format!("{}/manga/{}/", base_url, id)
            }
            Self::Search { by, query, page } => {
                format!("{}/manga-{}/{}/page/{}/", base_url, by, query, page)
            }
            Self::Filter {
                query,
                op,
                genre,
                author,
                artist,
                release,
                adult,
                status,
                m_orderby,
                page,
            } => {
                let mut genre_str = String::new();
                for genre in genre {
                    genre_str.push_str(&Self::uri_encode("genre".to_string(), genre.clone(), None));
                }

                let mut status_str = String::new();
                for status in status {
                    status_str.push_str(&Self::uri_encode(
                        "status".to_string(),
                        status.clone(),
                        None,
                    ));
                }

                let mut page_str = String::new();
                if *page > 1 {
                    page_str.push_str(&format!("/page/{}", page));
                }

                format!(
                    "{}{}/?s={}&op={}&post_type=wp-manga&author={}&artist={}&release={}&adult={}&m_orderby={}{}{}",
                    base_url,
                    page_str,
                    query,
                    op,
                    author,
                    artist,
                    release,
                    adult,
                    m_orderby,
                    genre_str,
                    status_str
                )
            }
        }
    }

    pub fn filters(query: Option<&str>, page: i32, filters: &[FilterValue]) -> Result<Self> {
        let mut op = String::from("");
        let mut genre = Vec::new();
        let mut author = String::from("");
        let mut artist = String::from("");
        let mut release = String::from("");
        let mut adult = String::from("");
        let mut status = Vec::new();
        let mut m_orderby = String::from("");

        for filter in filters {
            match filter {
                FilterValue::Text { id, value } => match id.as_str() {
                    "作者" => author = value.clone(),
                    "畫家" => artist = value.clone(),
                    "發布年分" => release = value.clone(),
                    "author" => {
                        return Ok(Self::Search {
                            by: "author".to_string(),
                            query: value.clone(),
                            page,
                        });
                    }
                    _ => continue,
                },
                FilterValue::Sort { index, .. } => {
                    m_orderby = match index {
                        1 => "latest".to_string(),
                        2 => "rating".to_string(),
                        3 => "views".to_string(),
                        5 => "new-manga".to_string(),
                        _ => "".to_string(),
                    };
                }
                FilterValue::MultiSelect {
                    id,
                    included,
                    excluded,
                    ..
                } => match id.as_str() {
                    "類型" => {
                        for tag in included {
                            genre.push(tag.clone());
                        }
                        for tag in excluded {
                            genre.retain(|x| x != tag);
                        }
                    }
                    "狀態" => {
                        for s in included {
                            status.push(s.clone());
                        }
                        for s in excluded {
                            status.retain(|x| x != s);
                        }
                    }
                    _ => continue,
                },
                FilterValue::Select { id, value } => match id.as_str() {
                    "條件" => op = value.clone(),
                    "內容" => adult = value.clone(),
                    "genre" => {
                        return Ok(Self::Search {
                            by: "tag".to_string(),
                            query: value.clone(),
                            page,
                        });
                    }
                    _ => continue,
                },
                _ => continue,
            }
        }

        Ok(Self::Filter {
            query: query.unwrap_or_default().to_string(),
            op,
            genre,
            author,
            artist,
            release,
            adult,
            status,
            m_orderby,
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
