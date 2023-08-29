use crate::service_net::PacketType;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::ServiceNetRs;

/// Connection id
#[derive(Copy, Clone, PartialEq, Eq, std::hash::Hash)]
#[repr(C)]
pub struct ConnId {
    pub id: usize,
    // TODO: add self as payload to EndPoint
}

impl ConnId {
    ///
    #[inline(always)]
    pub fn send(&self, srv_net: &ServiceNetRs, data: &[u8]) {
        let hd = *self;

        // 在当前线程中加 read 锁取出 conn，以便尽快发送
        let conn_opt = srv_net.get_conn(hd);
        if let Some(conn) = conn_opt {
            conn.send(data);
        } else {
            log::error!("[hd={}] send failed!!!", hd);
        }
    }

    /// Drop the tcp conn, and check if auto reconnect
    pub fn close(self, srv_net: &ServiceNetRs) {
        let hd = self;
        log::info!("[hd={}] close ...", hd);

        // 在当前线程中加 write 锁取出 conn
        let conn_opt = srv_net.get_conn(hd);
        if let Some(conn) = conn_opt {
            // low level close
            conn.close();

            // remove conn at once
            srv_net.remove_conn(hd);
        } else {
            log::error!("[hd={}] close failed!!!", hd);
        }
    }

    ///
    pub fn to_socket_addr(&self, srv_net: &ServiceNetRs) -> Option<SocketAddr> {
        let hd = *self;

        // 在当前线程中加 read 锁取出 conn，这样方便返回数值
        let conn_opt = srv_net.get_conn(hd);
        if let Some(conn) = conn_opt {
            Some(conn.endpoint.addr())
        } else {
            None
        }
    }

    ///
    pub fn set_packet_type(&self, srv_net: &ServiceNetRs, packet_type: PacketType) {
        let hd = *self;

        // 在当前线程中加 read 锁取出 conn
        let conn_opt = srv_net.get_conn(hd);
        if let Some(conn) = conn_opt {
            conn.set_packet_type(packet_type);
        } else {
            log::error!("[hd={}] change pakcet type failed!!!", hd);
        }
    }
}

impl From<usize> for ConnId {
    #[inline(always)]
    fn from(raw: usize) -> Self {
        Self { id: raw }
    }
}

// 为了使用 `{}` 标记，必须手动为类型实现 `fmt::Display` trait。
impl std::fmt::Display for ConnId {
    // 这个 trait 要求 `fmt` 使用与下面的函数完全一致的函数签名
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // 仅将 self 的第一个元素写入到给定的输出流 `f`。返回 `fmt:Result`，此
        // 结果表明操作成功或失败。注意 `write!` 的用法和 `println!` 很相似。
        write!(f, "{}", self.id)
    }
}
