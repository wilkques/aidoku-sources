use aidoku::{
    Result,
    alloc::String,
    imports::net::{HttpMethod, Request},
    prelude::*,
};

use crate::settings;

pub struct Fetch;

impl Fetch {
    pub fn request(url: String, method: HttpMethod) -> Result<Request> {
        Ok(Request::new(url.clone(), method)?
            // .header("User-Agent", &settings::get_user_agent())
            // .header("Origin", &settings::get_base_url())
            .header("Referer", &format!("{}/", settings::get_base_url())))
    }

    pub fn get(url: String) -> Result<Request> {
        Fetch::request(url, HttpMethod::Get)
    }
}
