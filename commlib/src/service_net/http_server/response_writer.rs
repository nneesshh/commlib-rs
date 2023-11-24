use std::borrow::Borrow;
use std::sync::Arc;

use net_packet::take_large_packet;

use crate::service_net::TcpConn;

#[inline(always)]
pub fn write_response<T: Borrow<[u8]>>(response: http::Response<T>, conn: &Arc<TcpConn>) {
    let (parts, body) = response.into_parts();
    let body: &[u8] = body.borrow();

    const HEADER_SIZE_MAX: usize = 4096; // should we use 4k header?
    let mut resp_buffer = take_large_packet(HEADER_SIZE_MAX + body.len());

    resp_buffer.append_slice(
        std::format!(
            "HTTP/1.1 {} {}\r\n",
            parts.status.as_str(),
            parts
                .status
                .canonical_reason()
                .expect("Unsupported HTTP Status"),
        )
        .as_bytes(),
    );

    if !parts.headers.contains_key(http::header::DATE) {
        let now = chrono::Utc::now();
        resp_buffer.append_slice(
            std::format!("{}\r\n", now.format("%a, %d %b %Y %H:%M:%S GMT")).as_bytes(),
        );
    }
    if !parts.headers.contains_key(http::header::CONNECTION) {
        resp_buffer.append_slice(b"connection: close\r\n");
    }
    if !parts.headers.contains_key(http::header::CONTENT_LENGTH) {
        resp_buffer.append_slice(std::format!("content-length: {}\r\n", body.len()).as_bytes());
    }
    for (k, v) in parts.headers.iter() {
        match v.to_str() {
            Ok(s) => {
                //
                resp_buffer.append_slice(std::format!("{}: {}\r\n", k.as_str(), s).as_bytes());
            }
            Err(_) => {
                //
            }
        }
    }

    resp_buffer.append_slice(b"\r\n"); // http EOF
    resp_buffer.append_slice(body); // http content

    conn.send_buffer(resp_buffer);
    conn.close();
}
