use std::net::SocketAddr;
use std::sync::Arc;

use crate::{ServiceNetRs, ServiceRs};

use super::NetPacketGuard;

/// Connection id
#[derive(Copy, Clone, PartialEq, Eq, std::hash::Hash)]
#[repr(C)]
pub struct ConnId {
    pub id: usize,
    // TODO: add self as payload to EndPoint
}

impl ConnId {
    /// disconnect - drop the TcpConn
    pub fn disconnet(self, srv_net: &Arc<ServiceNetRs>) {
        let hd = self;
        log::info!("[hd={}] disconnect ...", hd);

        //
        let srv_net2 = srv_net.clone();
        srv_net.run_in_service(Box::new(move || {
            let mut conn_table = srv_net2.conn_table.write();
            if let Some(conn) = conn_table.get_mut(&hd) {
                conn.disconnect();
            } else {
                log::error!("[hd={}] send failed -- hd not found!!!", hd);
            }
            conn_table.remove(&hd);
        }));
    }

    ///
    #[inline(always)]
    pub fn send(&self, srv_net: &Arc<ServiceNetRs>, data: &[u8]) {
        let hd = *self;

        //
        {
            let conn_table = srv_net.conn_table.read();
            if let Some(conn) = conn_table.get(&hd) {
                conn.send(data);
            } else {
                log::error!("[hd={}] send failed -- hd not found!!!", hd);
            }
        }
    }

    ///
    #[inline(always)]
    pub fn send_proto<M>(&self, srv_net: &Arc<ServiceNetRs>, msg: &M)
    where
        M: prost::Message,
    {
        let hd = *self;
        let vec = msg.encode_to_vec();

        //
        {
            let conn_table = srv_net.conn_table.read();
            if let Some(conn) = conn_table.get(&hd) {
                conn.send(vec.as_slice());
            } else {
                log::error!("[hd={}] send_proto failed -- hd not found!!!", hd);
            }
        }
    }

    ///
    pub fn to_socket_addr(&self, srv_net: &Arc<ServiceNetRs>) -> Option<SocketAddr> {
        let hd = *self;

        //
        {
            let conn_table = srv_net.conn_table.read();
            if let Some(conn) = conn_table.get(&hd) {
                Some(conn.endpoint.addr())
            } else {
                log::error!("[hd={}] to_socket_addr failed -- hd not found!!!", hd);
                None
            }
        }
    }

    /// call conn_fn
    pub fn run_conn_fn(&self, srv: &Arc<dyn ServiceRs>, srv_net: &Arc<ServiceNetRs>) {
        let hd = *self;

        //
        let f_opt = {
            let conn_table = srv_net.conn_table.read();
            if let Some(conn) = conn_table.get(&hd) {
                Some(conn.conn_fn.clone())
            } else {
                None
            }
        };

        //
        srv.run_in_service(Box::new(move || {
            if let Some(f) = f_opt {
                (f)(hd);
            } else {
                log::error!("[hd={}][run_conn_fn] failed!!!", hd);
            }
        }));
    }

    /// call pkt_fn
    pub fn run_pkt_fn(
        &self,
        srv: &Arc<dyn ServiceRs>,
        srv_net: &Arc<ServiceNetRs>,
        pkt: NetPacketGuard,
    ) {
        let hd = *self;

        //
        let f_opt = {
            let conn_table = srv_net.conn_table.read();
            if let Some(conn) = conn_table.get(&hd) {
                Some(conn.pkt_fn.clone())
            } else {
                None
            }
        };

        //
        srv.run_in_service(Box::new(move || {
            if let Some(f) = f_opt {
                (f)(hd, pkt);
            } else {
                log::error!("[hd={}][run_pkt_fn] failed!!!", hd);
            }
        }));
    }

    /// call close_fn
    pub fn run_close_fn(&self, srv: &Arc<dyn ServiceRs>, srv_net: &Arc<ServiceNetRs>) {
        let hd = *self;

        //
        let f_opt = {
            let conn_table = srv_net.conn_table.read();
            if let Some(conn) = conn_table.get(&hd) {
                Some(conn.close_fn.clone())
            } else {
                None
            }
        };

        //
        let srv_net2 = srv_net.clone();
        srv.run_in_service(Box::new(move || {
            if let Some(f) = f_opt {
                (f)(hd);
            } else {
                log::error!("[hd={}][run_close_fn] failed!!!", hd);
            }

            // disconnect - drop the connection
            hd.disconnet(&srv_net2);
        }));
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
