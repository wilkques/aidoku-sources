use aidoku::{
    Result,
    alloc::String,
    imports::net::{HttpMethod, Request},
};

use crate::settings;

pub struct Fetch;

impl Fetch {
    pub fn request(url: String, method: HttpMethod) -> Result<Request> {
        let user_agent = settings::get_user_agent();

        Ok(Request::new(url, method)?
            .header("User-Agent", &user_agent)
            .header("Cookie", "isAdult=1"))
    }

    pub fn get(url: String) -> Result<Request> {
        Fetch::request(url, HttpMethod::Get)
    }
}
