use aidoku::{
    alloc::string::String,
    imports::defaults::{DefaultValue, defaults_get, defaults_set},
};

const BASE_URL_KEY: &str = "url";
const USER_AGENT_KEY: &str = "userAgent";

pub fn get_base_url() -> String {
    let mut base_url = defaults_get::<String>(BASE_URL_KEY).unwrap_or_default();

    if base_url.is_empty() {
        let default_base_url = "https://www.dm5.cn";

        defaults_set(
            BASE_URL_KEY,
            DefaultValue::String(String::from(default_base_url)),
        );

        base_url = String::from(default_base_url);
    }

    base_url
}

pub fn get_user_agent() -> String {
    let mut user_agent = defaults_get::<String>(USER_AGENT_KEY).unwrap_or_default();

    if user_agent.is_empty() {
        let default_user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36";

        defaults_set(
            USER_AGENT_KEY,
            DefaultValue::String(String::from(default_user_agent)),
        );

        user_agent = String::from(default_user_agent);
    }

    user_agent
}
