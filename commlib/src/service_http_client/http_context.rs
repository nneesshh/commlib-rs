//!
//! Commlib: HttpContext
//!

use super::{HttpRequest, HttpResponse};

///
pub struct HttpContext {
    pub request: HttpRequest,
    pub response: HttpResponse,
}

impl HttpContext {
    ///
    pub fn new(request: HttpRequest) -> Self {
        Self {
            request: request,
            response: HttpResponse::new(),
        }
    }
}
