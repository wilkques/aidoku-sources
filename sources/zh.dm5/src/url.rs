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

const OPTIONS_TAGS: &[&str] = &[
    "全部",
    "热血",
    "恋爱",
    "校园",
    "百合",
    "彩虹",
    "冒险",
    "后宫",
    "科幻",
    "战争",
    "悬疑",
    "推理",
    "搞笑",
    "奇幻",
    "魔法",
    "恐怖",
    "神鬼",
    "历史",
    "同人",
    "运动",
    "绅士",
    "机甲",
    "限制级",
];

const OPTIONS_AREAS: &[&str] = &["全部", "港台", "日韩", "大陆", "欧美"];

const OPTIONS_AUDIENCES: &[&str] = &["全部", "少年向", "少女向", "青年向"];

const OPTIONS_WORDS: &[&str] = &[
    "全部", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q",
    "R", "S", "T", "U", "V", "W", "X", "Y", "Z", "0-9",
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
                FilterValue::Sort { index, .. } => {
                    sort = match index {
                        0 => "-s2".to_string(),
                        2 => "-s18".to_string(),
                        _ => "".to_string(),
                    };
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
                    "受眾" => audience = value.clone(),
                    "收費" => {
                        pay = if value.is_empty() {
                            String::new()
                        } else {
                            format!("-{}", value.clone())
                        }
                    }
                    "字母" => word = value.clone(),
                    "genre" => tag = value.clone(),
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

        // 透過 options 的 index 查找對應的 ids 或內部 id
        let convert = |value: &str, options: &[&str], target_ids: &[&str]| {
            options
                .iter()
                .position(|&opt| opt == value)
                .and_then(|idx| target_ids.get(idx))
                .map(|&s| s.to_string())
                .unwrap_or_default()
        };

        // 根據 active 數量決定轉換到外部 id (`rexue`) 還是內部 id (`tag31`)
        if active > 0 {
            let (tags, areas, audiences, words) = if active == 1 {
                (FILTER_TAGS, FILTER_AREAS, FILTER_AUDIENCES, FILTER_WORDS)
            } else {
                (TAGS, AREAS, AUDIENCES, WORDS)
            };

            tag = convert(&tag, OPTIONS_TAGS, tags);
            area = convert(&area, OPTIONS_AREAS, areas);
            audience = convert(&audience, OPTIONS_AUDIENCES, audiences);
            word = convert(&word, OPTIONS_WORDS, words);
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
