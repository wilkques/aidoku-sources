use aidoku::{
    Result,
    alloc::String,
    imports::net::{HttpMethod, Request},
    prelude::*,
};

use crate::settings::get_base_url;

pub struct Fetch;

impl Fetch {
    pub fn request(url: String, method: HttpMethod) -> Result<Request> {
        Ok(Request::new(url, method)?.header("Referer", &format!("{}/", get_base_url())))
    }

    pub fn get(url: String) -> Result<Request> {
        Fetch::request(url, HttpMethod::Get)
    }
}
