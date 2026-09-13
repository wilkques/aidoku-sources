use aidoku::{
    alloc::string::String,
    imports::defaults::{DefaultValue, defaults_get, defaults_set},
};

const USER_AGENT_KEY: &str = "userAgent";

pub fn get_base_url() -> String {
    String::from("https://m.hipmh.com")
}

pub fn get_api_url() -> String {
    String::from("https://hipapi1.s3file.top")
}

// 閱讀器是完全不同的網域，不是 base_url (m.hipmh.com/chapter/{id} 其實是首頁)
pub fn get_reader_url() -> String {
    String::from("https://reader.hipmh.top")
}

pub fn get_user_agent() -> String {
    let mut user_agent = defaults_get::<String>(USER_AGENT_KEY).unwrap_or_default();

    if user_agent.is_empty() {
        let default_user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36";

        defaults_set(USER_AGENT_KEY, DefaultValue::String(String::from(default_user_agent)));

        user_agent = String::from(default_user_agent);
    }

    user_agent
}
