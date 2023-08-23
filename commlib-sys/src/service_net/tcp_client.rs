//! Commlib: TcpClient
//! We can use this class to create a TCP client.
//! The typical usage is :
//!      1. Create a TcpClient object
//!      2. Set the message callback and connection callback
//!      3. Call TcpClient::Connect() to try to establish a connection with remote server
//!      4. Call TcpClient::Send(...) to send message to remote server
//!      5. Handle the connection and message in callbacks
//!      6. Call TcpClient::Disconnect() to disconnect from remote server
//!

use parking_lot::RwLock;
use std::sync::Arc;

use crate::{ServiceNetRs, ServiceRs};

use super::{ClientStatus, ConnId, NetPacketGuard, TcpConn};

///
#[repr(C)]
pub struct TcpClient {
    start: std::time::Instant,
    status: RwLock<ClientStatus>,

    //
    pub name: String,
    pub raddr: String,
    pub id: ConnId,

    //
    pub srv: Arc<dyn ServiceRs>,
    pub conn_fn: Arc<dyn Fn(ConnId) + Send + Sync>,
    pub pkt_fn: Arc<dyn Fn(ConnId, NetPacketGuard) + Send + Sync>,
    pub close_fn: Arc<dyn Fn(ConnId) + Send + Sync>,

    //
    inner_conn_opt: Arc<RwLock<Option<TcpConn>>>,
}

impl TcpClient {
    ///
    pub fn new<T>(name: &str, raddr: &str, srv: &Arc<T>) -> TcpClient
    where
        T: ServiceRs + 'static,
    {
        Self {
            start: std::time::Instant::now(),
            status: RwLock::new(ClientStatus::Null),

            name: name.to_owned(),
            raddr: raddr.to_owned(),
            id: ConnId::from(0),

            srv: srv.clone(),
            conn_fn: Arc::new(|_hd| {}),
            pkt_fn: Arc::new(|_hd, _pkt| {}),
            close_fn: Arc::new(|_hd| {}),

            inner_conn_opt: Arc::new(RwLock::new(None)),
        }
    }

    /// Connect to [ip:port]
    pub fn connect(&self, srv_net: &Arc<ServiceNetRs>) {
        //super::message_io_connect();
    }

    /// Stop net event loop
    pub fn stop(&self) {
        //self.inner_stop();
    }

    ///
    pub fn set_connection_callback<F>(&mut self, cb: F)
    where
        F: Fn(ConnId) + Send + Sync + 'static,
    {
        self.conn_fn = Arc::new(cb);
    }

    ///
    pub fn set_message_callback<F>(&mut self, cb: F)
    where
        F: Fn(ConnId, NetPacketGuard) + Send + Sync + 'static,
    {
        self.pkt_fn = Arc::new(cb);
    }

    ///
    pub fn set_close_callback<F>(&mut self, cb: F)
    where
        F: Fn(ConnId) + Send + Sync + 'static,
    {
        self.close_fn = Arc::new(cb);
    }
}
