//! Commlib: TcpServer
//! We can use this class to create a TCP server.
//! The typical usage is :
//!      1. Create a TcpServer object
//!      2. Set the message callback and connection callback
//!      3. Call TcpServer::Init()
//!      4. Call TcpServer::Start()
//!      5. Process TCP client connections and messages in callbacks
//!      6. At last call Server::Stop() to stop the whole server
//!

use atomic::{Atomic, Ordering};
use std::net::SocketAddr;
use std::sync::Arc;

use super::MessageIoNetwork;
use super::{ConnId, NetPacketGuard, ServerStatus, TcpConn, TcpListenerId};

use crate::{ServiceNetRs, ServiceRs};

///
#[repr(C)]
pub struct TcpServer {
    start: std::time::Instant,
    status: Atomic<ServerStatus>,

    connection_limit: Atomic<usize>,
    connection_num: Atomic<usize>,

    //
    pub addr: String,
    pub listener_id: TcpListenerId,
    pub listen_fn: Arc<dyn Fn(SocketAddr, ServerStatus) + Send + Sync>,

    //
    pub srv: Arc<dyn ServiceRs>,
    pub mi_network: Arc<MessageIoNetwork>,
    pub srv_net: Arc<ServiceNetRs>,

    //
    conn_fn: Arc<dyn Fn(Arc<TcpConn>) + Send + Sync>,
    pkt_fn: Arc<dyn Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync>,
    close_fn: Arc<dyn Fn(ConnId) + Send + Sync>,
}

impl TcpServer {
    ///
    pub fn new<T, C, P, S>(
        srv: &Arc<T>,
        addr: &str,
        conn_fn: C,
        pkt_fn: P,
        close_fn: S,
        mi_network: &Arc<MessageIoNetwork>,
        srv_net: &Arc<ServiceNetRs>,
    ) -> TcpServer
    where
        T: ServiceRs + 'static,
        C: Fn(Arc<TcpConn>) + Send + Sync + 'static,
        P: Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync + 'static,
        S: Fn(ConnId) + Send + Sync + 'static,
    {
        Self {
            start: std::time::Instant::now(),
            status: Atomic::new(ServerStatus::Null),

            connection_limit: Atomic::new(0_usize),
            connection_num: Atomic::new(0_usize),

            addr: addr.to_owned(),
            listener_id: TcpListenerId::from(0),
            listen_fn: Arc::new(|_sock_addr, _status| {}),

            srv: srv.clone(),
            mi_network: mi_network.clone(),
            srv_net: srv_net.clone(),

            conn_fn: Arc::new(conn_fn),
            pkt_fn: Arc::new(pkt_fn),
            close_fn: Arc::new(close_fn),
        }
    }

    /// Create a tcp server and listen on [ip:port]
    pub fn listen(&mut self) {
        self.set_status(ServerStatus::Starting);
        log::info!(
            "tcp server start listen at addr: {}, status: {}",
            self.addr,
            self.status().to_string()
        );

        self.listen_fn = Arc::new(|sock_addr, status| {
            //
            log::info!(
                "tcp server listen at {:?} success, status:{}",
                sock_addr,
                status.to_string()
            );
        });

        // inner server listen
        let mi_network = self.mi_network.clone();
        if !mi_network.listen(self) {
            self.set_status(ServerStatus::Down);

            //
            log::info!(
                "tcp server listen at {:?} failed!!! status:{}!!!",
                self.addr,
                self.status().to_string()
            );
        }
    }

    ///
    pub fn stop(&mut self) {
        // TODO:
    }

    ///
    #[inline(always)]
    pub fn status(&self) -> ServerStatus {
        self.status.load(Ordering::Relaxed)
    }

    ///
    #[inline(always)]
    pub fn set_status(&self, status: ServerStatus) {
        self.status.store(status, Ordering::Relaxed);
    }

    ///
    #[inline(always)]
    pub fn clone_conn_fn(&self) -> Arc<dyn Fn(Arc<TcpConn>) + Send + Sync> {
        self.conn_fn.clone()
    }

    ///
    #[inline(always)]
    pub fn clone_pkt_fn(&self) -> Arc<dyn Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync> {
        self.pkt_fn.clone()
    }

    ///
    #[inline(always)]
    pub fn clone_close_fn(&self) -> Arc<dyn Fn(ConnId) + Send + Sync> {
        self.close_fn.clone()
    }

    ///
    pub fn setup_callbacks<C, P, S>(&mut self, conn_fn: C, pkt_fn: P, close_fn: S)
    where
        C: Fn(Arc<TcpConn>) + Send + Sync + 'static,
        P: Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync + 'static,
        S: Fn(ConnId) + Send + Sync + 'static,
    {
        self.conn_fn = Arc::new(conn_fn);
        self.pkt_fn = Arc::new(pkt_fn);
        self.close_fn = Arc::new(close_fn);
    }
}
