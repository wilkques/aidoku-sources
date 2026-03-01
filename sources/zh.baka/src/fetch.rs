use aidoku::{
    Result,
    alloc::String,
    imports::net::{HttpMethod, Request},
};

pub struct Fetch;

impl Fetch {
    pub fn request(url: String, method: HttpMethod) -> Result<Request> {
        Ok(Request::new(url, method)?)
    }

    pub fn get(url: String) -> Result<Request> {
        Fetch::request(url, HttpMethod::Get)
    }
}
