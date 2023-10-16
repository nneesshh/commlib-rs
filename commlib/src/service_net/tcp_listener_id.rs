use std::net::SocketAddr;

use super::{ServerStatus, TcpServer};

/// Tcp server id
#[derive(Copy, Clone, PartialEq, Eq, std::hash::Hash)]
#[repr(C)]
pub struct TcpListenerId {
    pub id: usize,
    // TODO: add self as payload to EndPoint
}

impl TcpListenerId {
    /// Trigger listen_fn of tcp server
    pub fn run_listen_fn(&self, tcp_server: &mut TcpServer, sock_addr: SocketAddr) {
        let srv = tcp_server.srv().clone();
        let listener_id = *self;

        // update listener id
        tcp_server.listener_id = listener_id;

        // 状态：Running
        tcp_server.set_status(ServerStatus::Running);

        // post 到指定 srv 工作线程中执行
        let listen_fn_opt = Some(tcp_server.listen_fn.clone());
        let status = tcp_server.status();
        srv.run_in_service(Box::new(move || {
            if let Some(listen_fn) = listen_fn_opt {
                listen_fn(sock_addr, status);
            } else {
                log::error!("[listen_id={}][run_listen_fn] failed!!!", listener_id);
            }
        }));
    }
}

impl From<usize> for TcpListenerId {
    #[inline(always)]
    fn from(raw: usize) -> Self {
        Self { id: raw }
    }
}

impl std::fmt::Display for TcpListenerId {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
