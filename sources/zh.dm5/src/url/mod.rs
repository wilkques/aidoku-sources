use aidoku::{
    FilterValue, Result,
    alloc::{String, string::ToString as _},
    helpers::uri::encode_uri,
    prelude::*,
};

use crate::settings;

const TAGS: &[&str] = &[
    "", "tag31", "tag26", "tag1", "tag3", "tag27", "tag2", "tag8", "tag25", "tag12", "tag17",
    "tag33", "tag37", "tag14", "tag15", "tag29", "tag20", "tag4", "tag30", "tag34", "tag36",
    "tag40", "tag61",
];

const AREAS: &[&str] = &["", "area35", "area36", "area37", "area52"];

const AUDIENCES: &[&str] = &["", "group1", "group2", "group3"];

const WORDS: &[&str] = &[
    "", "charA", "charB", "charC", "charD", "charE", "charF", "charG", "charH", "charI", "charJ",
    "charK", "charL", "charM", "charN", "charO", "charP", "charQ", "charR", "charS", "charT",
    "charU", "charV", "charW", "charX", "charY", "charZ", "char0-9",
];

const FILTER_TAGS: &[&str] = &[
    "",
    "rexue",
    "aiqing",
    "xiaoyuan",
    "baihe",
    "caihong",
    "maoxian",
    "hougong",
    "kehuan",
    "zhanzheng",
    "xuanyi",
    "zhentan",
    "gaoxiao",
    "qihuan",
    "mofa",
    "kongbu",
    "dongfangshengui",
    "lishi",
    "tongren",
    "jingji",
    "jiecao",
    "jizhan",
    "tag61",
];

const FILTER_AREAS: &[&str] = &["", "hktw", "jpkr", "china", "euus"];

const FILTER_AUDIENCES: &[&str] = &["", "shaonan", "shaonv", "qingnian"];

const FILTER_WORDS: &[&str] = &[
    "", "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r",
    "s", "t", "u", "v", "w", "x", "y", "z", "0-9",
];

#[derive(Clone)]
pub enum Url {
    Filter {
        main_sort: String,
        sort: String,
        tag: String,
        area: String,
        status: String,
        audience: String,
        pay: String,
        word: String,
        page: String,
    },
    Search {
        query: String,
        page: i32,
    },
    Chapter {
        id: String,
    },
    Book {
        id: String,
    },
    Rank {
        id: String,
    },
}

impl Url {
    pub fn to_string(&self) -> String {
        let base_url = settings::get_base_url();

        match self {
            Self::Chapter { id } => {
                format!("{}/{}", base_url, id)
            }
            Self::Book { id } => {
                format!("{}/{}", base_url, id)
            }
            Self::Search { query, page } => {
                format!(
                    "{}/search?title={}&language=1&page={}",
                    base_url, query, page
                )
            }
            Self::Filter {
                main_sort,
                sort,
                tag,
                area,
                status,
                audience,
                pay,
                word,
                page,
            } => {
                format!(
                    "{}/manhua{}{}{}{}{}{}{}{}{}",
                    base_url, main_sort, area, word, tag, audience, pay, status, sort, page
                )
            }
            Self::Rank { id } => {
                if id != "new" {
                    return format!("{}/manhua-rank/?t={}", base_url, id);
                }

                format!("{}/manhua-{}", base_url, id)
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

        /// 透過 filter_ids 的 index 查找對應的內部 id
        fn convert(value: &str, filter_ids: &[&str], internal_ids: &[&str]) -> String {
            filter_ids
                .iter()
                .position(|&id| id == value)
                .and_then(|idx| internal_ids.get(idx))
                .map(|&s| s.to_string())
                .unwrap_or_default()
        }

        let mut sort = String::from("");
        let mut tag = String::from("");
        let mut area = String::from("");
        let mut status = String::from("");
        let mut audience = String::from("");
        let mut pay = String::from("");
        let mut word = String::from("");

        for filter in filters {
            match filter {
                FilterValue::Text { value, .. } => {
                    return Ok(Self::Search {
                        query: encode_uri(value.clone()),
                        page,
                    });
                }
                FilterValue::Select { id, value } => match id.as_str() {
                    "排序" => {
                        sort = if value.is_empty() {
                            String::new()
                        } else {
                            format!("-{}", value.clone())
                        }
                    }
                    "題材" => tag = value.clone(),
                    "地區" => area = value.clone(),
                    "進度" => {
                        status = if value.is_empty() {
                            String::new()
                        } else {
                            format!("-{}", value.clone())
                        }
                    }
                    "受眾" => audience = value.clone(),
                    "收費" => {
                        pay = if value.is_empty() {
                            String::new()
                        } else {
                            format!("-{}", value.clone())
                        }
                    }
                    "字母" => word = value.clone(),
                    _ => continue,
                },
                _ => continue,
            }
        }

        let page = format!("-p{}", page);

        // 計算有幾個非空的篩選條件
        let active = [&tag, &area, &status, &audience, &pay, &word]
            .iter()
            .filter(|v| !v.is_empty())
            .count();

        // 決定 sort 前綴 + 是否轉換
        let main_sort = match (sort.as_str(), active) {
            // (_, 0) => "-list".to_string(),                     // 全空 → "list"
            // (s, _) if !s.is_empty() => format!("-list-{}", s), // sort 非空 → "list-s2"
            (_, 1) => String::new(),  // 只有 1 個篩選，用 filter ids
            _ => "-list".to_string(), // 2+ 個篩選 → 固定 "list-"
        };

        // 2+ 個篩選條件 → 轉換成內部 id
        if active > 1 {
            tag = convert(&tag, FILTER_TAGS, TAGS);
            area = convert(&area, FILTER_AREAS, AREAS);
            audience = convert(&audience, FILTER_AUDIENCES, AUDIENCES);
            word = convert(&word, FILTER_WORDS, WORDS);
        }

        let prefix = |s: String| if s.is_empty() { s } else { format!("-{s}") };
        let tag = prefix(tag);
        let area = prefix(area);
        let audience = prefix(audience);
        let word = prefix(word);

        Ok(Self::Filter {
            main_sort,
            sort,
            tag,
            area,
            status,
            audience,
            pay,
            word,
            page,
        })
    }

    pub fn book(id: String) -> Result<Self> {
        Ok(Self::Book { id })
    }

    pub fn chapter(id: String) -> Result<Self> {
        Ok(Self::Chapter { id })
    }

    pub fn rank(id: String) -> Result<Self> {
        Ok(Self::Rank { id })
    }
}
