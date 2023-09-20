//! Commlib: RedisClient
//! We can use this class to create a TCP client.
//! The typical usage is :
//!      1. Create a RedisClient object
//!      2. Set the message callback and connection callback
//!      3. Call RedisClient::Connect() to try to establish a connection with remote server
//!      4. Call RedisClient::Send(...) to send message to remote server
//!      5. Handle the connection and message in callbacks
//!      6. Call RedisClient::Disconnect() to disconnect from remote server
//!

use atomic::{Atomic, Ordering};
use parking_lot::RwLock;
use std::sync::Arc;

use message_io::node::NodeHandler;

use crate::service_net::MessageIoNetwork;
use crate::service_net::{
    tcp_client_manager::tcp_client_reconnect, tcp_conn_manager::disconnect_connection,
};
use crate::{ClientStatus, ConnId, RedisReply, TcpConn};
use crate::{Clock, ServiceNetRs, ServiceRs, G_SERVICE_NET};

use super::RedisCommand;

/// Client for redis
#[repr(C)]
pub struct RedisClient {
    start: std::time::Instant,
    status: Atomic<ClientStatus>,

    //
    id: uuid::Uuid,
    name: String,
    raddr: String,
    pass: String,   // redis 密码
    dbindex: isize, // redis db index

    //
    srv: Arc<dyn ServiceRs>,
    mi_network: Arc<MessageIoNetwork>,
    srv_net: Arc<ServiceNetRs>,
    auto_reconnect: bool,

    //
    conn_fn: RwLock<Arc<dyn Fn(Arc<TcpConn>) + Send + Sync>>,
    reply_fn: RwLock<Arc<dyn Fn(Arc<TcpConn>, RedisReply) + Send + Sync>>,
    close_fn: RwLock<Arc<dyn Fn(ConnId) + Send + Sync>>,

    //
    inner_hd: Atomic<ConnId>,

    //
    redis_command: RedisCommand,
}

impl RedisClient {
    ///
    pub fn new<T>(
        srv: &Arc<T>,
        name: &str,
        raddr: &str,
        pass: &str,
        dbindex: isize,
        mi_network: &Arc<MessageIoNetwork>,
        srv_net: &Arc<ServiceNetRs>,
    ) -> RedisClient
    where
        T: ServiceRs + 'static,
    {
        Self {
            start: std::time::Instant::now(),
            status: Atomic::new(ClientStatus::Null),

            id: uuid::Uuid::new_v4(),

            name: name.to_owned(),
            raddr: raddr.to_owned(),
            pass: pass.to_owned(),
            dbindex,

            srv: srv.clone(),
            mi_network: mi_network.clone(),
            srv_net: srv_net.clone(),
            auto_reconnect: true,

            conn_fn: RwLock::new(Arc::new(|_1| {})),
            reply_fn: RwLock::new(Arc::new(|_1, _2| {})),
            close_fn: RwLock::new(Arc::new(|_1| {})),

            inner_hd: Atomic::new(ConnId::from(0)),

            redis_command: RedisCommand::new(
                std::format!("{}:{}", name, raddr).as_str(),
                pass,
                dbindex,
            ),
        }
    }

    /// Event: on_connect
    pub fn on_connect(&self, conn: Arc<TcpConn>) {
        //
        log::info!(
            "id<{}>[hd={}]({}) redis client on_connect ... raddr: {} status: {} -- conn: {}",
            self.id,
            self.inner_hd(),
            self.name,
            self.raddr,
            self.status().to_string(),
            conn.hd,
        );

        //
        //self.redis_command.on_link_connected(&conn);
    }

    /// Event: on_receive_reply
    pub fn on_receive_reply(&self, _conn: Arc<TcpConn>, _reply: RedisReply) {
        //
        log::info!(
            "id<{}>[hd={}]({}) redis client on_receive_reply ... raddr: {} status: {}",
            self.id,
            self.inner_hd(),
            self.name,
            self.raddr,
            self.status().to_string()
        );
    }

    /// Event: on_disconnect
    pub fn on_disconnect(&self, _hd: ConnId) {
        //
        log::info!(
            "id<{}>[hd={}]({}) redis client on_disconnect ... raddr: {} status: {}",
            self.id,
            self.inner_hd(),
            self.name,
            self.raddr,
            self.status().to_string()
        );
    }

    /// Connect to [ip:port]
    pub fn connect(&self) -> Result<ConnId, String> {
        log::info!(
            "[hd={}]({}) redis client start connect to raddr: {} status: {} -- id<{}>",
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
        match mi_network.redis_client_connect(self) {
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
            "[hd={}]({}) redis client start reconnect to raddr: {} status: {} -- id<{}>",
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
                "[hd={}]({}) redis client reconnect to raddr: {} status: {} -- id<{}> falied: {}!!!",
                self.inner_hd(),
                self.name,
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
            "[hd={}]({}) redis client try to reconnect after {}ms -- id<{}> ...",
            self.inner_hd(),
            self.name,
            DELAY_MS,
            self.id
        );

        let cli_id = self.id;
        let hd = self.inner_hd();
        let name = self.name.clone();

        let srv_net = self.srv_net.clone();

        //
        Clock::set_timeout(self.srv_net.as_ref(), DELAY_MS, move || {
            log::info!(
                "[hd={}]({}) redis client reconnect -- id<{}> ...",
                hd,
                name,
                cli_id
            );
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
            "[hd={}]({}) redis client start disconnect from raddr: {} status: {} -- id<{}>",
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
                "[hd={}]({}) redis client disconnect from raddr: {} status: {} -- id<{}> falied: {}!!!",
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
            "[hd={}]({}) redis client disconnecting from raddr: {} status: {} -- id<{}>",
            inner_hd,
            self.name,
            self.raddr,
            self.status().to_string(),
            self.id,
        );

        //
        let cb = move |hd| {
            log::info!("[hd={}] redis client disconnect over.", hd);
            disconneced_cb(hd);
        };
        disconnect_connection(&G_SERVICE_NET, inner_hd, cb, &self.srv);

        //
        Ok(())
    }

    /// inner hd
    pub fn inner_hd(&self) -> ConnId {
        self.inner_hd.load(Ordering::Relaxed)
    }

    ///
    pub fn set_inner_hd(&self, hd: ConnId) {
        self.inner_hd.store(hd, Ordering::Relaxed)
    }

    /// status
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
    #[inline(always)]
    pub fn id(&self) -> uuid::Uuid {
        self.id
    }

    ///
    #[inline(always)]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    ///
    #[inline(always)]
    pub fn remote_addr(&self) -> &str {
        self.raddr.as_str()
    }

    ///
    #[inline(always)]
    pub fn netctrl(&self) -> &NodeHandler<()> {
        &self.mi_network.node_handler
    }

    ///
    #[inline(always)]
    pub fn srv_net(&self) -> &Arc<ServiceNetRs> {
        &self.srv_net
    }

    ///
    #[inline(always)]
    pub fn srv(&self) -> &Arc<dyn ServiceRs> {
        &self.srv
    }

    ///
    #[inline(always)]
    pub fn clone_conn_fn(&self) -> Arc<dyn Fn(Arc<TcpConn>) + Send + Sync> {
        let conn_fn = self.conn_fn.read();
        conn_fn.clone()
    }

    ///
    #[inline(always)]
    pub fn clone_reply_fn(&self) -> Arc<dyn Fn(Arc<TcpConn>, RedisReply) + Send + Sync> {
        let reply_fn = self.reply_fn.read();
        reply_fn.clone()
    }

    ///
    #[inline(always)]
    pub fn clone_close_fn(&self) -> Arc<dyn Fn(ConnId) + Send + Sync> {
        let close_fn = self.close_fn.read();
        close_fn.clone()
    }

    ///
    pub fn setup_callbacks<C, R, S>(&self, conn_fn: C, reply_fn: R, close_fn: S)
    where
        C: Fn(Arc<TcpConn>) + Send + Sync + 'static,
        R: Fn(Arc<TcpConn>, RedisReply) + Send + Sync + 'static,
        S: Fn(ConnId) + Send + Sync + 'static,
    {
        // conn_fn
        {
            let mut conn_fn_mut = self.conn_fn.write();
            (*conn_fn_mut) = Arc::new(conn_fn);
        }

        // reply_fn
        {
            let mut reply_fn_mut = self.reply_fn.write();
            (*reply_fn_mut) = Arc::new(reply_fn);
        }

        // close_fn
        {
            let mut close_fn_mut = self.close_fn.write();
            (*close_fn_mut) = Arc::new(close_fn);
        }
    }

    ///
    pub fn check_auto_reconnect(&self) {
        if self.auto_reconnect {
            //
            match self.reconnect() {
                Ok(_) => {
                    log::error!(
                        "[hd={}]({}) redis client auto reconnect start -- id<{}> ...",
                        self.inner_hd(),
                        self.name,
                        self.id,
                    );
                }
                Err(err) => {
                    log::error!(
                        "[hd={}]({}) redis client auto reconnect -- id<{}> failed: {}!!!",
                        self.inner_hd(),
                        self.name,
                        self.id,
                        err
                    );
                }
            }
        }
    }

    ///
    pub fn command(&self) -> &RedisCommand {
        &self.redis_command
    }
}
