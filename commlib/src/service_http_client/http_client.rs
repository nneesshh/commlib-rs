//!
//! Commlib: HttpClient
//!

use atomic::{Atomic, Ordering};
use parking_lot::RwLock;
use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::sync::Arc;

use curl::easy::{Easy as CurlEasy, List as CurlList};
use curl::multi::{EasyHandle, Multi as CurlMulti};

use crate::{ServiceHttpClientRs, ServiceRs};

use super::{HttpContext, HttpRequest, HttpRequestType};

static NEXT_TOKEN_ID: Atomic<usize> = Atomic::<usize>::new(1);

thread_local! {
    static G_HTTP_CLIENT: UnsafeCell<HttpClient> = UnsafeCell::new(HttpClient::new());

    static G_CURL_PAYLOAD_STORAGE: UnsafeCell<CurlPayloadStorage> = UnsafeCell::new(CurlPayloadStorage::new());
}

struct CurlPayload {
    easy_handle: EasyHandle, // EasyHandle owns raw pointer, can send across thread
    context: Arc<RwLock<HttpContext>>,
}

struct CurlPayloadStorage {
    /// custom handle table
    payload_table: hashbrown::HashMap<usize, CurlPayload>, // token 2 payload
}

impl CurlPayloadStorage {
    ///
    pub fn new() -> Self {
        Self {
            payload_table: hashbrown::HashMap::with_capacity(256),
        }
    }
}

///
pub struct HttpClient {
    request_queue: VecDeque<HttpRequest>,
    multi_handler: CurlMulti,
}

impl HttpClient {
    ///
    pub fn new() -> Self {
        Self {
            request_queue: VecDeque::with_capacity(64),
            multi_handler: CurlMulti::new(),
        }
    }

    ///
    pub fn send(&mut self, req: HttpRequest, srv_http_cli: &ServiceHttpClientRs) {
        // 运行于 srv_http_cli 线程
        assert!(srv_http_cli.is_in_service_thread());

        self.enqueue(req);
    }

    ///
    pub fn run_loop(&mut self, srv_http_cli: &ServiceHttpClientRs) {
        // 运行于 srv_http_cli 线程
        assert!(srv_http_cli.is_in_service_thread());

        // process requests
        const MAX_REQUESTS: usize = 100_usize;
        let mut count = 0_usize;
        while count <= MAX_REQUESTS {
            if let Some(req) = self.request_queue.pop_front() {
                //
                let context = Arc::new(RwLock::new(HttpContext::new(req)));

                // 设置 easy
                let mut easy = CurlEasy::new();
                configure_easy(&context, &mut easy).unwrap();

                // handle 交给 multi_handler 处理
                let mut easy_handle = self.multi_handler.add(easy).unwrap();

                // easy handle <-- token
                let token = NEXT_TOKEN_ID.fetch_add(1, Ordering::Relaxed);
                easy_handle.set_token(token).unwrap();

                // insert easy handle
                insert_curl_payload(
                    srv_http_cli,
                    token,
                    CurlPayload {
                        easy_handle,
                        context,
                    },
                );
            } else {
                break;
            }

            //
            count += 1;
        }

        // perform
        loop {
            //
            match self.multi_handler.perform() {
                Ok(num) => {
                    if num > 0 {
                        self.multi_handler
                            .wait(&mut [], std::time::Duration::from_millis(100))
                            .unwrap();
                    } else {
                        //
                        break;
                    }
                }
                Err(multi_error) => {
                    // CURLM_CALL_MULTI_PERFORM 需要 perform again
                    if !multi_error.is_call_perform() {
                        //
                        break;
                    }
                }
            }
        }

        // response
        self.multi_handler.messages(|msg| {
            // token
            let token = msg.token().unwrap();

            // 根据 token 查找 payload
            let payload_opt = remove_curl_payload(srv_http_cli, token);
            if let Some(mut payload) = payload_opt {
                //
                let easy_handle = &mut payload.easy_handle;
                let mut context_mut = payload.context.write();

                let msg_result_opt = msg.result_for(&easy_handle);
                if let Some(msg_result) = msg_result_opt {
                    //
                    match msg_result {
                        Ok(()) => {
                            // resp code == 200 成功
                            let resp_code = easy_handle.response_code().unwrap();
                            if resp_code == 200 {
                                // success
                                context_mut.response.response_code = resp_code;
                                context_mut.response.succeed = true;

                                //
                                (context_mut.request.request_cb)(&context_mut);
                            }
                        }
                        Err(error) => {
                            //
                            log::error!("multi_handler message failed!!! error:{error}!!!");
                        }
                    }
                } else {
                    //
                    log::error!("multi_handler message failed!!! msg handle not valid!!!");
                }
            } else {
                log::error!(
                    "multi_handler message failed!!! invalid token: {}!!!",
                    token
                );
            }
        });
    }

    #[inline(always)]
    fn enqueue(&mut self, req: HttpRequest) {
        self.request_queue.push_back(req);
    }
}

///
pub fn http_client_update(srv_http_cli: &ServiceHttpClientRs) {
    // 运行于 srv_http_cli 线程
    assert!(srv_http_cli.is_in_service_thread());

    with_tls_mut!(G_HTTP_CLIENT, g, {
        g.run_loop(srv_http_cli);
    });
}

fn configure_easy(
    context: &Arc<RwLock<HttpContext>>,
    easy: &mut CurlEasy,
) -> Result<(), curl::Error> {
    //
    let context_ = context.read();
    let req = &context_.request;

    // configure timeout, ssl verify, signal ...
    {
        easy.timeout(std::time::Duration::from_secs(30))?;
        easy.connect_timeout(std::time::Duration::from_secs(10))?;
        easy.ssl_verify_peer(true)?;
        easy.ssl_verify_host(true)?;
        easy.signal(true)?;
    }

    // 设置 headers
    if !req.headers.is_empty() {
        let mut header_list = CurlList::new();
        for header in &req.headers {
            header_list.append(header.as_str())?;
        }
        easy.http_headers(header_list)?;
    }

    // 设置 url
    easy.url(req.url.as_str())?;

    //
    match req.r#type {
        HttpRequestType::GET => {
            easy.follow_location(true)?;
            easy.custom_request("GET")?;
            easy.post(false)?;
        }
        HttpRequestType::POST => {
            easy.custom_request("POST")?;
            easy.post(true)?;

            easy.post_fields_copy(req.reuqest_data.as_bytes())?;
            easy.post_field_size(req.reuqest_data.len() as u64)?;
        }
        HttpRequestType::PUT => {
            easy.custom_request("PUT")?;
            easy.post(false)?;

            easy.post_fields_copy(req.reuqest_data.as_bytes())?;
            easy.post_field_size(req.reuqest_data.len() as u64)?;
        }
        HttpRequestType::DEL => {
            easy.custom_request("DELETE")?;
            easy.post(false)?;
            easy.follow_location(true)?;
        }
        HttpRequestType::UNKNOWN => {
            //
            log::error!("unkonwn http request type:{}", req.r#type as u8);
        }
    }

    // 设置 write callback
    let context2 = context.clone();
    easy.write_function(move |data: &[u8]| {
        //
        let s = unsafe { std::str::from_utf8_unchecked(data.into()) };

        //
        {
            let mut context_mut = context2.write();
            context_mut.response.response_rawdata.push_str(s);
        }

        //
        Ok(data.len())
    })?;

    // 设置 header callback
    let context2 = context.clone();
    easy.header_function(move |data: &[u8]| {
        //
        let s = unsafe { std::str::from_utf8_unchecked(data.into()) };

        //
        {
            let mut context_mut = context2.write();
            context_mut.response.response_headers.push(s.to_owned());
        }

        //
        true
    })?;

    //
    Ok(())
}

fn insert_curl_payload(
    srv_http_cli: &ServiceHttpClientRs,
    token: usize,
    curl_payload: CurlPayload,
) {
    // 运行于 srv_http_cli 线程
    assert!(srv_http_cli.is_in_service_thread());

    with_tls_mut!(G_CURL_PAYLOAD_STORAGE, g, {
        log::info!("insert_curl_payload token: {}", token);
        g.payload_table.insert(token, curl_payload);
    });
}

fn remove_curl_payload(srv_http_cli: &ServiceHttpClientRs, token: usize) -> Option<CurlPayload> {
    // 运行于 srv_http_cli 线程
    assert!(srv_http_cli.is_in_service_thread());

    with_tls_mut!(G_CURL_PAYLOAD_STORAGE, g, {
        log::info!("remove_curl_payload token: {}", token);
        g.payload_table.remove(&token)
    })
}

#[cfg(test)]
mod http_test {
    use serde_json::json;

    use crate::G_SERVICE_HTTP_CLIENT;

    #[test]
    fn curl_test() {
        let body = json!({"foo": false, "bar": null, "answer": 42, "list": [null, "world", true]});

        //
        let srv_http_cli = G_SERVICE_HTTP_CLIENT.clone();
    }
}
