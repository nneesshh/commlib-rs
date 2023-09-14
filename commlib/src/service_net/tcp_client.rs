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

use atomic::{Atomic, Ordering};
use std::sync::Arc;

use crate::{Clock, ServiceNetRs, ServiceRs, G_SERVICE_NET};

use super::{tcp_client_manager::tcp_client_reconnect, tcp_conn_manager::disconnect_connection};
use super::{ClientStatus, ConnId, MessageIoNetwork, NetPacketGuard, TcpConn};

///
#[repr(C)]
pub struct TcpClient {
    start: std::time::Instant,
    status: Atomic<ClientStatus>,

    //
    pub id: uuid::Uuid,
    pub name: String,
    pub raddr: String,

    //
    pub srv: Arc<dyn ServiceRs>,
    pub mi_network: Arc<MessageIoNetwork>,
    pub srv_net: Arc<ServiceNetRs>,
    pub auto_reconnect: bool,

    //
    pub conn_fn: Arc<dyn Fn(Arc<TcpConn>) + Send + Sync>,
    pub pkt_fn: Arc<dyn Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync>,
    pub close_fn: Arc<dyn Fn(ConnId) + Send + Sync>,

    //
    pub inner_hd: Atomic<ConnId>,
}

impl TcpClient {
    ///
    pub fn new<T, C, P, S>(
        srv: &Arc<T>,
        name: &str,
        raddr: &str,
        mi_network: &Arc<MessageIoNetwork>,
        conn_fn: C,
        pkt_fn: P,
        close_fn: S,
        srv_net: &Arc<ServiceNetRs>,
    ) -> TcpClient
    where
        T: ServiceRs + 'static,
        C: Fn(Arc<TcpConn>) + Send + Sync + 'static,
        P: Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync + 'static,
        S: Fn(ConnId) + Send + Sync + 'static,
    {
        Self {
            start: std::time::Instant::now(),
            status: Atomic::new(ClientStatus::Null),

            id: uuid::Uuid::new_v4(),

            name: name.to_owned(),
            raddr: raddr.to_owned(),

            srv: srv.clone(),
            mi_network: mi_network.clone(),
            srv_net: srv_net.clone(),
            auto_reconnect: true,

            conn_fn: Arc::new(conn_fn),
            pkt_fn: Arc::new(pkt_fn),
            close_fn: Arc::new(close_fn),

            inner_hd: Atomic::new(ConnId::from(0)),
        }
    }

    ///
    pub fn inner_hd(&self) -> ConnId {
        self.inner_hd.load(Ordering::Relaxed)
    }

    ///
    pub fn set_inner_hd(&self, hd: ConnId) {
        self.inner_hd.store(hd, Ordering::Relaxed)
    }

    /// Connect to [ip:port]
    pub fn connect(&self) -> Result<ConnId, String> {
        log::info!(
            "[hd={}]({}) start connect to raddr: {} status: {} -- id<{}>",
            self.inner_hd(),
            self.name,
            self.raddr,
            self.status().to_string(),
            self.id,
        );

        // status: Connecting
        self.set_status(ClientStatus::Connecting);

        // inner connect
        let mi_network = self.mi_network.clone();
        match mi_network.connect(self) {
            Ok(hd) => {
                self.set_status(ClientStatus::Connected);
                Ok(hd)
            }
            Err(err) => {
                self.set_status(ClientStatus::Disconnected);

                // check auto reconnect
                self.check_auto_reconnect();
                Err(err)
            }
        }
    }

    /// Reonnect to [ip:port]
    pub fn reconnect(&self) -> Result<(), String> {
        log::info!(
            "[hd={}]({}) start reconnect to raddr: {} status: {} -- id<{}>",
            self.inner_hd(),
            self.name,
            self.raddr,
            self.status().to_string(),
            self.id,
        );

        // client 必须处于空闲状态
        if !self.status().is_idle() {
            let errmsg = "wrong status".to_owned();
            log::error!(
                "tcp client reconnect to raddr: {} status: {} -- id<{}> falied: {}!!!",
                self.raddr,
                self.status().to_string(),
                self.id,
                errmsg,
            );
            return Err(errmsg);
        }

        //
        const DELAY_MS: u64 = 5000_u64; // ms
        log::info!(
            "[hd={}]({}) try to reconnect after {}ms -- id<{}> ...",
            self.inner_hd(),
            self.name,
            DELAY_MS,
            self.id
        );

        let hd = self.inner_hd();
        let name = self.name.clone();
        let cli_id = self.id.clone();

        let srv_net = self.srv_net.clone();

        //
        Clock::set_timeout(self.srv_net.as_ref(), DELAY_MS, move || {
            log::info!("[hd={}]({}) reconnect -- id<{}> ...", hd, name, cli_id);
            {
                tcp_client_reconnect(&srv_net, hd, name, cli_id);
            }
        });

        //
        Ok(())
    }

    /// Disconnect client
    pub fn disconnect<F>(&mut self, disconneced_cb: F) -> Result<(), String>
    where
        F: Fn(ConnId) + Send + Sync + 'static,
    {
        let inner_hd = self.inner_hd();

        log::info!(
            "[hd={}]({}) start disconnect from raddr: {} status: {} -- id<{}>",
            inner_hd,
            self.name,
            self.raddr,
            self.status().to_string(),
            self.id,
        );

        // client 必须处于连接状态
        if !self.status().is_connected() {
            let errmsg = "wrong status".to_owned();
            log::error!(
                "[hd={}]({}) disconnect from raddr: {} status: {} -- id<{}> falied: {}!!!",
                inner_hd,
                self.name,
                self.raddr,
                self.status().to_string(),
                self.id,
                errmsg,
            );
            return Err(errmsg);
        }

        // remove inner TcpConn by hd
        self.set_status(ClientStatus::Disconnecting);
        log::info!(
            "[hd={}]({}) disconnecting from raddr: {} status: {} -- id<{}>",
            inner_hd,
            self.name,
            self.raddr,
            self.status().to_string(),
            self.id,
        );

        //
        let cb = move |hd| {
            log::info!("[hd={}] disconnect over.", hd);
            disconneced_cb(hd);
        };
        disconnect_connection(&G_SERVICE_NET, inner_hd, cb, &self.srv);

        //
        Ok(())
    }

    ///
    pub fn set_connection_callback<F>(&mut self, cb: F)
    where
        F: Fn(Arc<TcpConn>) + Send + Sync + 'static,
    {
        self.conn_fn = Arc::new(cb);
    }

    ///
    pub fn set_message_callback<F>(&mut self, cb: F)
    where
        F: Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync + 'static,
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
        self.status.load(Ordering::Relaxed)
    }

    ///
    #[inline(always)]
    pub fn set_status(&self, status: ClientStatus) {
        self.status.store(status, Ordering::Relaxed);
    }

    ///
    pub fn check_auto_reconnect(&self) {
        if self.auto_reconnect {
            //
            match self.reconnect() {
                Ok(_) => {
                    log::error!(
                        "[hd={}]({}) auto reconnect start -- id<{}> ...",
                        self.inner_hd(),
                        self.name,
                        self.id,
                    );
                }
                Err(err) => {
                    log::error!(
                        "[hd={}]({}) auto reconnect -- id<{}> failed: {}!!!",
                        self.inner_hd(),
                        self.name,
                        self.id,
                        err
                    );
                }
            }
        }
    }
}
