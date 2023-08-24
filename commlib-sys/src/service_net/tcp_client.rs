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

use super::{ClientStatus, ConnId, MessageIoNetwork, NetPacketGuard, TcpConn};

///
#[repr(C)]
pub struct TcpClient {
    start: std::time::Instant,
    status: ClientStatus,

    //
    pub conn_opt: Option<TcpConn>,

    //
    pub name: String,
    pub raddr: String,

    //
    pub srv: Arc<dyn ServiceRs>,
    pub mi_network: Arc<MessageIoNetwork>,
    pub srv_net: Arc<ServiceNetRs>,
    pub auto_reconnect: bool,

    //
    pub conn_fn: Arc<dyn Fn(ConnId) + Send + Sync>,
    pub pkt_fn: Arc<dyn Fn(ConnId, NetPacketGuard) + Send + Sync>,
    pub close_fn: Arc<dyn Fn(ConnId) + Send + Sync>,

    //
    pub inner_hd: ConnId,
}

impl TcpClient {
    ///
    pub fn new<T>(
        srv: &Arc<T>,
        name: &str,
        raddr: &str,
        mi_network: &Arc<MessageIoNetwork>,
        srv_net: &Arc<ServiceNetRs>,
    ) -> TcpClient
    where
        T: ServiceRs + 'static,
    {
        Self {
            start: std::time::Instant::now(),
            status: ClientStatus::Null,

            conn_opt: None,

            name: name.to_owned(),
            raddr: raddr.to_owned(),

            srv: srv.clone(),
            mi_network: mi_network.clone(),
            srv_net: srv_net.clone(),
            auto_reconnect: true,

            conn_fn: Arc::new(|_hd| {}),
            pkt_fn: Arc::new(|_hd, _pkt| {}),
            close_fn: Arc::new(|_hd| {}),

            inner_hd: ConnId::from(0),
        }
    }

    /// Connect to [ip:port]
    pub fn connect(&mut self) -> Result<ConnId, String> {
        log::info!(
            "tcp client connect to raddr: {} status: {}",
            self.raddr,
            self.status.to_string()
        );

        // status: Connecting
        self.set_status(ClientStatus::Connecting);

        // inner connect
        let mi_network = self.mi_network.clone();
        match (*mi_network).connect(self) {
            Ok(hd) => {
                self.set_status(ClientStatus::Connected);
                Ok(hd)
            }
            Err(err) => {
                self.set_status(ClientStatus::Disconnected);
                Err(err)
            }
        }
    }

    /// Reonnect to [ip:port]
    pub fn reconnect(&mut self) -> Result<ConnId, String> {
        log::info!(
            "tcp client reconnect to raddr: {} status: {}",
            self.raddr,
            self.status.to_string()
        );

        //self.srv_net.clock();

        // inner connect
        let mi_network = self.mi_network.clone();
        match (*mi_network).connect(self) {
            Ok(hd) => {
                self.set_status(ClientStatus::Connected);
                Ok(hd)
            }
            Err(err) => {
                self.set_status(ClientStatus::Disconnected);
                Err(err)
            }
        }
    }

    /// Disconnect client
    pub fn disconnect(&mut self) {
        // remove inner TcpConn by hd
        self.set_status(ClientStatus::Disconnecting);
        log::info!("[hd={}] disconnecting", self.inner_hd);

        self.inner_hd.disconnet(&self.srv_net);

        self.set_status(ClientStatus::Disconnecting);
        log::info!("[hd={}] disconnecting", self.inner_hd);

        self.inner_hd = ConnId::from(0);
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

    ///
    #[inline(always)]
    pub fn status(&self) -> ClientStatus {
        self.status
    }

    #[inline(always)]
    fn set_status(&mut self, status: ClientStatus) {
        self.status = status;
    }
}
