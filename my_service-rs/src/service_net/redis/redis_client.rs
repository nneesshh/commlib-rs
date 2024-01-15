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

use commlib::Clock;
use net_packet::NetPacketGuard;
use pinky_swear::PinkySwear;

use crate::service_net::connector::Connector;
use crate::service_net::low_level_network::MessageIoNetwork;
use crate::service_net::tcp_conn_manager::disconnect_connection;
use crate::{ClientStatus, ConnId, RedisReply, TcpConn};
use crate::{ServiceNetRs, ServiceRs, G_SERVICE_NET};

use super::redis_client_manager::{
    redis_client_check_auto_reconnect, redis_client_make_new_conn, redis_client_reconnect,
};
use super::reply_builder::ReplyBuilder;
use super::RedisCommander;

/// Client for redis
pub struct RedisClient {
    //
    status: Atomic<ClientStatus>,

    //
    id: uuid::Uuid,
    name: String,
    raddr: String,
    pass: String,   // redis 密码
    dbindex: isize, // redis db index

    //
    srv: Arc<dyn ServiceRs>,
    netctrl: Arc<MessageIoNetwork>,
    srv_net: Arc<ServiceNetRs>,

    //
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
        netctrl: &Arc<MessageIoNetwork>,
        conn_fn: C,
        close_fn: S,
        srv_net: &Arc<ServiceNetRs>,
    ) -> Self
    where
        C: Fn(Arc<TcpConn>) + Send + Sync + 'static,
        S: Fn(ConnId) + Send + Sync + 'static,
    {
        Self {
            status: Atomic::new(ClientStatus::Null),

            id: uuid::Uuid::new_v4(),

            name: name.to_owned(),
            raddr: raddr.to_owned(),
            pass: pass.to_owned(),
            dbindex,

            srv: srv.clone(),
            netctrl: netctrl.clone(),
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
        assert!(self.srv_net.is_in_service_thread());

        /*
        //
        log::info!(
            "[hd={}]({}) redis client on_receive_reply raddr: {} status: {} ... id<{}> reply={:?}",
            self.inner_hd(),
            self.name,
            self.raddr,
            self.status().to_string(),
            self.id,
            reply
        );
        */

        // commander receive reply
        self.get_commander().on_receive_reply(reply);
    }

    /// Event: on_connect
    pub fn on_ll_connect(self: &Arc<Self>, conn: Arc<TcpConn>) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        //
        log::info!(
            "[hd={}]({}) redis client on_ll_connect raddr: {} status: {} ... id<{}> conn={}",
            self.inner_hd(),
            self.name,
            self.raddr,
            self.status().to_string(),
            self.id,
            conn.hd
        );

        // commander link ok
        self.get_commander().on_connect(&conn);

        // post 到指定 srv 工作线程中执行
        let cli_conn_fn = self.conn_fn.clone();
        self.srv.run_in_service(Box::new(move || {
            (*cli_conn_fn)(conn);
        }));
    }

    ///
    pub fn on_ll_input(self: &Arc<Self>, conn: Arc<TcpConn>, input_buffer: NetPacketGuard) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        self.get_reply_builder().build(&conn, input_buffer);
    }

    /// Event: on_disconnect
    pub fn on_ll_disconnect(self: &Arc<Self>, hd: ConnId) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        //
        log::info!(
            "[hd={}]({}) redis client on_ll_disconnect raddr: {} status: {} ... id<{}>",
            self.inner_hd(),
            self.name,
            self.raddr,
            self.status().to_string(),
            self.id
        );

        // commander link broken
        self.get_commander().on_disconnect();

        // post 到指定 srv 工作线程中执行
        let cli_close_fn = self.close_fn.clone();
        let srv_net = self.srv_net.clone();
        let cli_id = self.id();
        self.srv.run_in_service(Box::new(move || {
            (*cli_close_fn)(hd);

            // check auto reconnect
            redis_client_check_auto_reconnect(hd, cli_id, &srv_net);
        }));
    }

    /// Connect to [ip:port]
    pub fn connect<F>(self: &Arc<Self>, cb: F)
    where
        F: Fn(Arc<Self>, Option<String>) + Send + Sync + 'static,
    {
        assert!(self.status() != ClientStatus::Connecting);

        log::info!(
            "id<{}>({}) redis client start connect to raddr: {} status: {} ...",
            self.id,
            self.name,
            self.raddr,
            self.status().to_string()
        );

        // status: Connecting
        self.set_status(ClientStatus::Connecting);

        //
        let cli = self.clone();

        //
        let connect_fn = move |r| {
            match r {
                Ok((hd, sock_addr)) => {
                    // make new connection
                    redis_client_make_new_conn(&cli, hd, sock_addr);

                    //
                    cli.set_status(ClientStatus::Connected);
                    cb(cli.clone(), None);
                }
                Err(err) => {
                    //
                    cli.set_status(ClientStatus::Disconnected);

                    log::error!(
                        "[hd={}]({}) redis client connect to raddr: {} failed!!! status: {} -- id<{}>!!! error: {}!!!",
                        cli.inner_hd(),
                        cli.name(),
                        cli.remote_addr(),
                        cli.status().to_string(),
                        cli.id(),
                        err
                    );

                    //
                    cb(cli.clone(), Some(err));

                    // check auto reconnect
                    cli.check_auto_reconnect();
                }
            }
        };

        // start connector
        let connector = Arc::new(Connector::new(
            self.name.as_str(),
            connect_fn,
            &self.netctrl,
            &self.srv_net,
        ));
        connector.start(self.raddr.as_str());
    }

    /// Reonnect to [ip:port]
    pub fn reconnect(&self) -> Result<(), String> {
        log::info!(
            "[hd={}]({}) redis client start reconnect to raddr: {} status: {} ... id<{}>",
            self.inner_hd(),
            self.name,
            self.raddr,
            self.status().to_string(),
            self.id
        );

        // client 必须处于空闲状态
        if !self.status().is_idle() {
            let errmsg = "wrong status".to_owned();
            log::error!(
                "[hd={}]({}) redis client reconnect to raddr: {} failed!!! status: {} -- id<{}>!!! error: {}!!!",
                self.inner_hd(),
                self.name,
                self.raddr,
                self.status().to_string(),
                self.id,
                errmsg
            );
            return Err(errmsg);
        }

        //
        const DELAY_MS: u64 = 5000_u64; // ms
        log::info!(
            "[hd={}]({}) redis client try to reconnect after {}ms ... id<{}>",
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
        Clock::set_timeout(DELAY_MS, move || {
            log::info!(
                "[hd={}]({}) redis client reconnect ... id<{}>",
                hd,
                name,
                cli_id
            );

            //
            redis_client_reconnect(hd, name.as_str(), cli_id, &srv_net);
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
            "[hd={}]({}) redis client start disconnect from raddr: {} status: {} ... id<{}>",
            inner_hd,
            self.name,
            self.raddr,
            self.status().to_string(),
            self.id
        );

        // client 必须处于连接状态
        if !self.status().is_connected() {
            let errmsg = "wrong status".to_owned();
            log::error!(
                "[hd={}]({}) redis client disconnect from raddr: {} failed!!! status: {} -- id<{}>!!! error: {}!!!",
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
            "[hd={}]({}) redis client disconnecting from raddr: {} status: {} ... id<{}>",
            inner_hd,
            self.name,
            self.raddr,
            self.status().to_string(),
            self.id
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
                    log::info!(
                        "[hd={}]({}) redis client auto reconnect start ... id<{}>",
                        self.inner_hd(),
                        self.name,
                        self.id
                    );
                }
                Err(err) => {
                    log::error!(
                        "[hd={}]({}) redis client auto reconnect failed!!! status: {} -- id<{}>!!! error: {}!!!",
                        self.inner_hd(),
                        self.name,
                        self.status().to_string(),
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
    pub fn srv(&self) -> &Arc<dyn ServiceRs> {
        &self.srv
    }

    ///
    #[inline(always)]
    pub fn netctrl(&self) -> &Arc<MessageIoNetwork> {
        &self.netctrl
    }

    ///
    #[inline(always)]
    pub fn srv_net(&self) -> &Arc<ServiceNetRs> {
        &self.srv_net
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
            commander.do_commit();
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
        assert!(self.srv_net.is_in_service_thread());

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
        assert!(self.srv_net.is_in_service_thread());

        let builder = self.tls_reply_builder.get_or(|| {
            let cli = self.clone();

            let build_cb = move |conn: Arc<TcpConn>, reply: RedisReply| {
                // 运行于 srv_net 线程
                assert!(conn.srv_net_opt.as_ref().unwrap().is_in_service_thread());
                cli.on_receive_reply(reply);
            };
            UnsafeCell::new(ReplyBuilder::new(Box::new(build_cb)))
        });
        unsafe { &mut *(builder.get()) }
    }
}
