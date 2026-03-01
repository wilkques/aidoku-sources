use aidoku::{
    alloc::string::String,
    imports::defaults::{DefaultValue, defaults_get, defaults_set},
};

const BASE_URL_KEY: &str = "url";

pub fn get_base_url() -> String {
    let mut base_url = defaults_get::<String>(BASE_URL_KEY).unwrap_or_default();

    if base_url.is_empty() {
        let default_base_url = "https://bakamh.ru";

        defaults_set(
            BASE_URL_KEY,
            DefaultValue::String(String::from(default_base_url)),
        );

        base_url = String::from(default_base_url);
    }

    base_url
}
