//! Commlib: TcpClient
//! We can use this class to create a TCP client.
//! The typical usage is :
//!      1. Create a TcpClient object
//!      2. Set the message callback and connection callback
//!      3. Call TcpClient::connect() to try to establish a connection with remote server
//!      4. Call TcpClient::send(...) to send message to remote server
//!      5. Handle the connection and message in callbacks
//!      6. Call TcpClient::disconnect() to disconnect from remote server
//!

use atomic::{Atomic, Ordering};
use std::cell::UnsafeCell;
use std::sync::Arc;
use thread_local::ThreadLocal;

use commlib::Clock;
use net_packet::NetPacketGuard;

use crate::service_net::packet_builder::tcp_packet_builder::PacketBuilder;
use crate::{ServiceNetRs, ServiceRs};

use super::connector::Connector;
use super::low_level_network::MessageIoNetwork;
use super::tcp_client_manager::{
    tcp_client_check_auto_reconnect, tcp_client_make_new_conn, tcp_client_reconnect,
};
use super::tcp_conn_manager::disconnect_connection;
use super::{ClientStatus, ConnId, TcpConn};

///
pub struct TcpClient {
    //
    status: Atomic<ClientStatus>,

    //
    id: uuid::Uuid,
    name: String,
    raddr: String,

    //
    srv: Arc<dyn ServiceRs>,
    netctrl: Arc<MessageIoNetwork>,
    srv_net: Arc<ServiceNetRs>,

    //
    auto_reconnect: bool,

    //
    inner_hd: Atomic<ConnId>,

    //
    conn_fn: Arc<dyn Fn(Arc<TcpConn>) + Send + Sync>,
    pkt_fn: Arc<dyn Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync>,
    close_fn: Arc<dyn Fn(ConnId) + Send + Sync>,

    //
    tls_pkt_builder: ThreadLocal<UnsafeCell<PacketBuilder>>,
}

impl TcpClient {
    ///
    pub fn new<T, C, P, S>(
        srv: &Arc<T>,
        name: &str,
        raddr: &str,
        netctrl: &Arc<MessageIoNetwork>,
        conn_fn: C,
        pkt_fn: P,
        close_fn: S,
        srv_net: &Arc<ServiceNetRs>,
    ) -> Self
    where
        T: ServiceRs + 'static,
        C: Fn(Arc<TcpConn>) + Send + Sync + 'static,
        P: Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync + 'static,
        S: Fn(ConnId) + Send + Sync + 'static,
    {
        Self {
            status: Atomic::new(ClientStatus::Null),

            id: uuid::Uuid::new_v4(),

            name: name.to_owned(),
            raddr: raddr.to_owned(),

            srv: srv.clone(),
            netctrl: netctrl.clone(),
            srv_net: srv_net.clone(),

            auto_reconnect: true,

            conn_fn: Arc::new(conn_fn),
            pkt_fn: Arc::new(pkt_fn),
            close_fn: Arc::new(close_fn),

            inner_hd: Atomic::new(ConnId::from(0)),

            tls_pkt_builder: ThreadLocal::new(),
        }
    }

    ///
    pub fn on_ll_connect(self: &Arc<Self>, conn: Arc<TcpConn>) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        //
        log::info!(
            "[hd={}]({}) tcp client on_ll_connect raddr: {} status: {} ... id<{}> conn={}",
            self.inner_hd(),
            self.name,
            self.raddr,
            self.status().to_string(),
            self.id,
            conn.hd
        );

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

        self.get_packet_builder().build(&conn, input_buffer);
    }

    ///
    pub fn on_ll_disconnect(self: &Arc<Self>, hd: ConnId) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        //
        log::info!(
            "[hd={}]({}) tcp client on_ll_disconnect raddr: {} status: {} ... id<{}>",
            self.inner_hd(),
            self.name,
            self.raddr,
            self.status().to_string(),
            self.id
        );

        // post 到指定 srv 工作线程中执行
        let cli_close_fn = self.close_fn.clone();
        let srv_net = self.srv_net.clone();
        let cli_id = self.id();
        self.srv.run_in_service(Box::new(move || {
            (*cli_close_fn)(hd);

            // check auto reconnect
            tcp_client_check_auto_reconnect(hd, cli_id, &srv_net);
        }));
    }

    /// Connect to [ip:port]
    pub fn connect<F>(self: &Arc<Self>, cb: F)
    where
        F: Fn(Arc<Self>, Option<String>) + Send + Sync + 'static,
    {
        assert!(self.status() != ClientStatus::Connecting);

        log::info!(
            "id<{}>({}) tcp client start connect to raddr: {} status: {} ...",
            self.id,
            self.name,
            self.raddr,
            self.status().to_string()
        );

        // status: Connecting
        self.set_status(ClientStatus::Connecting);

        //
        let cli = self.clone();

        let connect_fn = move |r| {
            match r {
                Ok((hd, sock_addr)) => {
                    // make new connection
                    tcp_client_make_new_conn(&cli, hd, sock_addr);

                    //
                    cli.set_status(ClientStatus::Connected);
                    cb(cli.clone(), None);
                }
                Err(err) => {
                    //
                    cli.set_status(ClientStatus::Disconnected);

                    log::error!(
                        "[hd={}]({}) tcp client connect to raddr: {} failed!!! status: {} -- id<{}>!!! error: {}!!!",
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
            "[hd={}]({}) tcp client start reconnect to raddr: {} status: {} ... id<{}>",
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
                "[hd={}]({}) tcp client reconnect to raddr: {} failed!!! status: {} -- id<{}>!!! error: {}!!!",
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
            "[hd={}]({}) tcp client try to reconnect after {}ms ... id<{}>",
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
                "[hd={}]({}) tcp client reconnect ... id<{}>",
                hd,
                name,
                cli_id
            );

            //
            tcp_client_reconnect(hd, name.as_str(), cli_id, &srv_net);
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
            "[hd={}]({}) tcp client start disconnect from raddr: {} status: {} ... id<{}>",
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
                "[hd={}]({}) tcp client disconnect from raddr: {} failed!!! status: {} -- id<{}>!!! error: {}!!!",
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
            "[hd={}]({}) tcp client disconnecting from raddr: {} status: {} ... id<{}>",
            inner_hd,
            self.name,
            self.raddr,
            self.status().to_string(),
            self.id
        );

        //
        let cb = move |hd| {
            log::info!("[hd={}] tcp client disconnect over.", hd);
            disconneced_cb(hd);
        };
        disconnect_connection(&self.srv, inner_hd, cb, &self.srv_net);

        //
        Ok(())
    }

    ///
    pub fn inner_hd(&self) -> ConnId {
        self.inner_hd.load(Ordering::Relaxed)
    }

    ///
    pub fn set_inner_hd(&self, hd: ConnId) {
        self.inner_hd.store(hd, Ordering::Relaxed)
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

    ///
    pub fn check_auto_reconnect(&self) {
        if self.auto_reconnect {
            //
            match self.reconnect() {
                Ok(_) => {
                    log::info!(
                        "[hd={}]({}) tcp client auto reconnect start ... id<{}>",
                        self.inner_hd(),
                        self.name,
                        self.id
                    );
                }
                Err(err) => {
                    log::error!(
                        "[hd={}]({}) tcp client auto reconnect failed!!! status: {} -- id<{}>!!! error: {}!!!",
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

    ////////////////////////////////////////////////////////////////

    #[inline(always)]
    fn get_packet_builder<'a>(self: &'a Arc<Self>) -> &'a mut PacketBuilder {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        let builder = self.tls_pkt_builder.get_or(|| {
            let srv = self.srv.clone();
            let pkt_fn = self.pkt_fn.clone();

            let build_cb = move |conn: Arc<TcpConn>, pkt: NetPacketGuard| {
                // 运行于 srv_net 线程
                assert!(conn.srv_net_opt.as_ref().unwrap().is_in_service_thread());

                // post 到指定 srv 工作线程中执行
                let pkt_fn2 = pkt_fn.clone();
                srv.run_in_service(Box::new(move || {
                    (*pkt_fn2)(conn, pkt);
                }));
            };
            UnsafeCell::new(PacketBuilder::new(Box::new(build_cb)))
        });
        unsafe { &mut *(builder.get()) }
    }
}
