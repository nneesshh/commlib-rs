//!
//! Common Library: service-http
//!

///
pub mod http_response;
pub use http_response::HttpResponse;

///
pub mod http_request;
pub use http_request::{HttpRequest, HttpRequestType};

///
pub mod http_context;
pub use http_context::HttpContext;

///
pub mod http_client;
pub use http_client::http_client_update;
pub use http_client::HttpClient;

///
pub mod service_http_client_impl;
pub use service_http_client_impl::ServiceHttpClientRs;
