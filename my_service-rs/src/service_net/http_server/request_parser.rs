use std::sync::Arc;

use net_packet::{take_large_packet, NetPacketGuard};

use crate::{ConnId, TcpConn};

use super::error;
use super::parsing;

const MAX_REQUEST_SIZE: usize = 1024 * 1024 * 20; // 20M

/// Request read result
pub enum RequestResult {
    Suspend,                       // 等待数据
    Ready(http::Request<Vec<u8>>), // 数据完整
    Abort(String),
}

/// Reader state
enum RequestParserState {
    Null,          // 空包
    Data,          // 包体数据区
    Expand(usize), // 扩展包体缓冲区（request_full_len）
    Abort,         // 中止
}

///
pub struct RequestParser {
    state: RequestParserState,

    //
    pkt_opt: Option<NetPacketGuard>, // 使用 option 以便 pkt 移交
    parse_cb: Box<dyn Fn(Arc<TcpConn>, http::Request<Vec<u8>>) + Send + Sync>,
}

impl RequestParser {
    ///
    #[inline(always)]
    pub fn new(parse_cb: Box<dyn Fn(Arc<TcpConn>, http::Request<Vec<u8>>) + Send + Sync>) -> Self {
        Self {
            state: RequestParserState::Null,
            pkt_opt: None,
            parse_cb,
        }
    }

    /// 解析数据包，触发数据包回调函数
    #[inline(always)]
    pub fn parse(&mut self, conn: &Arc<TcpConn>, input_buffer: NetPacketGuard) {
        //
        match self.parse_once(conn.hd, input_buffer) {
            RequestResult::Suspend => {
                // TODO: 数据不完整? 超时?
                log::error!("waiting");
            }

            RequestResult::Ready(req) => {
                // trigger parse_cb
                (self.parse_cb)(conn.clone(), req);
            }

            RequestResult::Abort(err) => {
                //
                log::error!("[hd={}] parse request failed!!! error: {}", conn.hd, err);

                // low level close
                conn.close();
            }
        }
    }

    /* input_buffer 中存放 input 数据，一次性处理完毕，Ok 返回 req, Err 返回错误信息 */
    #[inline(always)]
    fn parse_once(&mut self, _hd: ConnId, input_buffer: NetPacketGuard) -> RequestResult {
        //
        let input_len = input_buffer.buffer_raw_len();
        let mut consumed = 0_usize;
        let mut remain = input_len;

        // debug only
        /* {
            let input = input_buffer.peek();
            let input_str = unsafe { std::str::from_utf8_unchecked(input) };
            log::info!(
                "[hd={}] input: ({}){:?} -- {}",
                _hd,
                input.len(),
                input,
                input_str
            );
            let input_hex = hex::encode(input);
            log::info!("[hd={}] input_hex: ({}) --> {}", _hd, input.len(), input_hex);
        }*/

        // input buffer 在循环中可能被 move，编译器会因为后续还有使用而报错，因此采用 option trick 封装一下
        let mut in_buf_opt = Some(input_buffer);

        // 解析 req
        let req = loop {
            //
            match self.state {
                RequestParserState::Null => {
                    let in_buf = in_buf_opt.take().unwrap();
                    let append_bytes = in_buf.buffer_raw_len();

                    // 初始化 pkt，直接使用 in_buf 避免 copy
                    self.pkt_opt = Some(in_buf);

                    // state: 进入包体前导长度读取
                    self.state = RequestParserState::Data;

                    //
                    remain -= append_bytes;
                    consumed += append_bytes;
                }

                RequestParserState::Data => {
                    let pkt = self.pkt_opt.as_mut().unwrap();
                    let buffer_raw_len = pkt.buffer_raw_len();

                    // 包体大小是否超过限制
                    let full_len = buffer_raw_len + remain;
                    if full_len >= MAX_REQUEST_SIZE {
                        // state: 中止
                        self.state = RequestParserState::Abort;
                    } else if remain > 0 {
                        // 检查 pkt 容量是否足够
                        let writable_bytes = pkt.free_space();
                        if writable_bytes < remain {
                            // state: 进入扩展包体缓冲区处理，重新申请 large pkt
                            self.state = RequestParserState::Expand(full_len);
                        } else {
                            // in_buf 中还有 input 数据未处理，将此 input 数据附加到内部 pkt
                            let append_bytes = remain;

                            // in_buf 中还有 input 数据未处理，将此 input 数据附加到内部 pkt
                            let mut in_buf = in_buf_opt.take().unwrap();
                            let to_append = in_buf.consume();
                            assert!(to_append.len() == append_bytes);
                            pkt.append_slice(to_append);

                            //
                            remain -= append_bytes;
                            consumed += append_bytes;
                        }
                    } else {
                        // 解析 request
                        let peek = pkt.peek();
                        let ret = parsing::try_parse_request(peek);
                        match ret {
                            Ok(parsing_result) => {
                                //
                                match parsing_result {
                                    parsing::ParseResult::Complete(req) => {
                                        // 返回完整数据
                                        assert!(consumed <= input_len);
                                        break req;
                                    }
                                    parsing::ParseResult::Partial => {
                                        // 数据不完整, 等待
                                        return RequestResult::Suspend;
                                    }
                                }
                            }

                            Err(_error) => {
                                // 数据不完整, 等待
                                return RequestResult::Suspend;
                            }
                        }
                    }
                }

                RequestParserState::Expand(request_full_len) => {
                    let old_pkt = self.pkt_opt.as_mut().unwrap();

                    //
                    let ensure_bytes = request_full_len;
                    let old_pkt_slice = old_pkt.consume();

                    // old pkt 数据转移到 new pkt，使用 new pkt 继续解析
                    let mut new_pkt = take_large_packet(ensure_bytes);
                    new_pkt.append_slice(old_pkt_slice);
                    self.pkt_opt = Some(new_pkt);

                    // 进入包体数据处理
                    self.state = RequestParserState::Data;
                }

                RequestParserState::Abort => {
                    log::error!("request overflow!!!");

                    // 包长度越界
                    return RequestResult::Abort("overflow".to_owned());
                }
            }
        };

        //
        match build_request(req) {
            Ok(request) => {
                //
                RequestResult::Ready(request)
            }

            Err(e) => {
                //
                RequestResult::Abort(std::format!("{:?}", e))
            }
        }
    }
}

#[inline(always)]
fn build_request(mut req: parsing::Request) -> Result<http::Request<Vec<u8>>, error::Error> {
    let mut http_req = http::Request::builder().method(req.method());

    for header in req.headers() {
        http_req = http_req.header(header.name, header.value);
    }

    let mut request = http_req.body(req.split_body())?;
    let path = req.path();
    *request.uri_mut() = path.parse()?;

    Ok(request)
}
