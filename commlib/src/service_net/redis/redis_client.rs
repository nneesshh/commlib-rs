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
use std::cell::UnsafeCell;
use std::sync::Arc;
use thread_local::ThreadLocal;

use message_io::node::NodeHandler;

use crate::service_net::redis::redis_client_manager::{
    redis_client_check_auto_reconnect, redis_client_reconnect,
};
use crate::service_net::tcp_conn_manager::disconnect_connection;
use crate::service_net::MessageIoNetwork;

use crate::PinkySwear;
use crate::{ClientStatus, ConnId, NetPacketGuard, RedisReply, TcpConn};
use crate::{Clock, ServiceNetRs, ServiceRs, G_SERVICE_NET};

use super::reply_builder::ReplyBuilder;
use super::RedisCommander;

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
    conn_fn: Arc<dyn Fn(Arc<TcpConn>) + Send + Sync>,
    close_fn: Arc<dyn Fn(ConnId) + Send + Sync>,

    //
    inner_hd: Atomic<ConnId>,

    //
    tls_redis_commander: ThreadLocal<UnsafeCell<RedisCommander>>,
    tls_reply_builder: ThreadLocal<UnsafeCell<ReplyBuilder>>,
}

impl RedisClient {
    ///
    pub fn new<C, S>(
        srv: &Arc<dyn ServiceRs>,
        name: &str,
        raddr: &str,
        pass: &str,
        dbindex: isize,
        mi_network: &Arc<MessageIoNetwork>,
        conn_fn: C,
        close_fn: S,
        srv_net: &Arc<ServiceNetRs>,
    ) -> Self
    where
        C: Fn(Arc<TcpConn>) + Send + Sync + 'static,
        S: Fn(ConnId) + Send + Sync + 'static,
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

            conn_fn: Arc::new(conn_fn),
            close_fn: Arc::new(close_fn),

            inner_hd: Atomic::new(ConnId::from(0)),

            tls_redis_commander: ThreadLocal::new(),
            tls_reply_builder: ThreadLocal::new(),
        }
    }

    /// Event: on_receive_reply
    #[inline(always)]
    pub fn on_receive_reply(self: &Arc<Self>, reply: RedisReply) {
        // 运行于 srv_net 线程
        assert!(self.srv_net().is_in_service_thread());

        //
        /*log::info!(
            "id<{}>[hd={}]({}) redis client on_receive_reply ... raddr: {} status: {}",
            self.id,
            self.inner_hd(),
            self.name,
            self.raddr,
            self.status().to_string()
        );*/

        // commander receive reply
        self.get_commander().on_receive_reply(reply);
    }

    /// Event: on_connect
    pub fn on_ll_connect(self: &Arc<Self>, conn: Arc<TcpConn>) {
        // 运行于 srv_net 线程
        assert!(self.srv_net().is_in_service_thread());

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

        // commander link ok
        self.get_commander().on_connect(&conn);

        // post 到指定 srv 工作线程中执行
        let cli_conn_fn = self.conn_fn.clone();
        self.srv().run_in_service(Box::new(move || {
            (*cli_conn_fn)(conn);
        }));
    }

    ///
    pub fn on_ll_input(self: &Arc<Self>, conn: Arc<TcpConn>, input_buffer: NetPacketGuard) {
        // 运行于 srv_net 线程
        assert!(self.srv_net().is_in_service_thread());

        self.get_reply_builder().build(&conn, input_buffer);
    }

    /// Event: on_disconnect
    pub fn on_ll_disconnect(self: &Arc<Self>, hd: ConnId) {
        // 运行于 srv_net 线程
        assert!(self.srv_net().is_in_service_thread());

        //
        log::info!(
            "id<{}>[hd={}]({}) redis client on_disconnect ... raddr: {} status: {}",
            self.id,
            self.inner_hd(),
            self.name,
            self.raddr,
            self.status().to_string()
        );

        // commander link broken
        self.get_commander().on_disconnect();

        // post 到指定 srv 工作线程中执行
        let cli_close_fn = self.close_fn.clone();
        let srv_net = self.srv_net().clone();
        let cli_id = self.id();
        self.srv().run_in_service(Box::new(move || {
            (*cli_close_fn)(hd);

            // check auto reconnect
            redis_client_check_auto_reconnect(&srv_net, hd, cli_id);
        }));
    }

    /// Connect to [ip:port]
    pub fn connect(self: &Arc<Self>) -> Result<ConnId, String> {
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
                redis_client_reconnect(&srv_net, hd, name.as_str(), cli_id);
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
        disconnect_connection(&self.srv, inner_hd, cb, &G_SERVICE_NET);

        //
        Ok(())
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
    pub fn is_connected(&self) -> bool {
        self.status() == ClientStatus::Connected
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

    /// Send redis command
    pub fn send<F>(self: &Arc<Self>, cmd: Vec<String>, cb: F)
    where
        F: FnOnce(RedisReply) + Send + Sync + 'static,
    {
        // 投递到 srv_net 线程
        let cli = self.clone();
        let srv = self.srv.clone();
        self.srv_net.run_in_service(Box::new(move || {
            //
            let commander = cli.get_commander();
            commander.do_send(cmd, move |_commander, reply| {
                // post 到指定 srv 工作线程中执行
                srv.run_in_service(Box::new(move || {
                    cb(reply);
                }))
            });
        }));
    }

    /// 异步提交： 如果提交失败，则在连接成功后再提交一次
    pub fn commit(self: &Arc<Self>) {
        // 投递到 srv_net 线程
        let cli = self.clone();
        self.srv_net.run_in_service(Box::new(move || {
            //
            let commander = cli.get_commander();
            commander.do_commit();
        }));
    }

    ///
    pub fn send_and_commit_blocking(self: &Arc<Self>, cmd: Vec<String>) -> PinkySwear<RedisReply> {
        // MUST NOT in srv_net thread，防止 blocking 导致死锁
        assert!(!self.srv_net.is_in_service_thread());

        let (prms, pinky) = PinkySwear::<RedisReply>::new();

        // 投递到 srv_net 线程
        let cli = self.clone();
        self.srv_net.run_in_service(Box::new(move || {
            //
            let commander = cli.get_commander();
            commander.do_send(cmd, move |_commander, reply| {
                pinky.swear(reply);
            });
        }));

        prms
    }

    /// 用户手工调用，清除 commands 缓存
    pub fn clear_commands(self: &Arc<Self>) {
        // 投递到 srv_net 线程
        let cli = self.clone();
        self.srv_net.run_in_service(Box::new(move || {
            //
            let commander = cli.get_commander();
            commander.do_clear_commands();
        }));
    }

    ////////////////////////////////////////////////////////////////

    #[inline(always)]
    fn get_commander<'a>(self: &'a Arc<Self>) -> &'a mut RedisCommander {
        // 运行于 srv_net 线程
        assert!(self.srv_net().is_in_service_thread());

        let commander = self.tls_redis_commander.get_or(|| {
            //
            UnsafeCell::new(RedisCommander::new(
                std::format!("@({})", self.name).as_str(),
                self.pass.as_str(),
                self.dbindex,
            ))
        });
        unsafe { &mut *(commander.get()) }
    }

    #[inline(always)]
    fn get_reply_builder<'a>(self: &'a Arc<Self>) -> &'a mut ReplyBuilder {
        // 运行于 srv_net 线程
        assert!(self.srv_net().is_in_service_thread());

        let builder = self.tls_reply_builder.get_or(|| {
            let cli = self.clone();

            let build_cb = move |conn: Arc<TcpConn>, reply: RedisReply| {
                // 运行于 srv_net 线程
                assert!(conn.srv_net.is_in_service_thread());
                cli.on_receive_reply(reply);
            };
            UnsafeCell::new(ReplyBuilder::new(Box::new(build_cb)))
        });
        unsafe { &mut *(builder.get()) }
    }
}
