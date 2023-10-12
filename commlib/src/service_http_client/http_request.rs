//!
//! Commlib: HttpRequest
//!

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
    pub url: String,          // target url that this request is sent to
    pub reuqest_data: String, // used for POST
    pub tag: String, // user defined tag, to identify different requests in response callback
    pub headers: Vec<String>, // custom http headers

    pub error_buffer: String, // if response_code != 200, please read error_buffer to find the reason
    pub request_rawdata: String, // the request raw data

    pub request_cb: Box<dyn Fn(&HttpContext) + Send + Sync>,
}

impl HttpRequest {
    ///
    pub fn new<F>(request_cb: F) -> Self
    where
        F: Fn(&HttpContext) + Send + Sync + 'static,
    {
        Self {
            r#type: HttpRequestType::GET,
            url: "".to_owned(),
            reuqest_data: "".to_owned(),
            tag: "".to_owned(),
            headers: Vec::with_capacity(32),

            error_buffer: "".to_owned(),
            request_rawdata: "".to_owned(),

            request_cb: Box::new(request_cb),
        }
    }
}
