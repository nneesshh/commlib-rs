//!
//! Commlib: HttpRequest
//!

use std::sync::Arc;

use super::HttpContext;

///
#[derive(PartialEq, Copy, Clone)]
#[repr(i8)]
pub enum HttpRequestType {
    GET,
    POST,
    PUT,
    DEL,
    UNKNOWN,
}

///
#[repr(C)]
pub struct HttpRequest {
    pub r#type: HttpRequestType,
    pub url: String,              // target url that this request is sent to
    pub data_opt: Option<String>, // used for POST
    pub headers: Vec<String>,     // custom http headers

    pub request_cb: Arc<dyn Fn(&mut HttpContext) + Send + Sync>,
}
