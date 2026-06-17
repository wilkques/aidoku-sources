use aidoku::{
    Result,
    alloc::{String, string::ToString as _},
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

    pub fn post(url: String) -> Result<Request> {
        Fetch::request(url, HttpMethod::Post)
    }
}
