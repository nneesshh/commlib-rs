//!
//! Commlib: TcpServer
//!

use atomic::{Atomic, Ordering};
use std::net::SocketAddr;
use std::sync::Arc;

use net_packet::NetPacketGuard;

use crate::service_net::listener::Listener;
use crate::service_net::low_level_network::MessageIoNetwork;
use crate::service_net::packet_builder::tcp_packet_builder::PacketBuilder;
use crate::{ConnId, ListenerId, ServerStatus, TcpConn};
use crate::{ServiceNetRs, ServiceRs};

use super::tcp_server_manager::tcp_server_make_new_conn;

///
pub struct TcpServer {
    //
    status: Atomic<ServerStatus>,

    //
    pub addr: String,

    //
    connection_limit: usize,
    connection_num: Atomic<usize>,

    //
    srv: Arc<dyn ServiceRs>,
    netctrl: Arc<MessageIoNetwork>,
    srv_net: Arc<ServiceNetRs>,

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
        connection_limit: usize,
        netctrl: &Arc<MessageIoNetwork>,
        srv_net: &Arc<ServiceNetRs>,
    ) -> Self
    where
        T: ServiceRs + 'static,
        C: Fn(Arc<TcpConn>) + Send + Sync + 'static,
        P: Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync + 'static,
        S: Fn(ConnId) + Send + Sync + 'static,
    {
        Self {
            status: Atomic::new(ServerStatus::Null),

            addr: addr.to_owned(),

            connection_limit,
            connection_num: Atomic::new(0_usize),

            srv: srv.clone(),
            netctrl: netctrl.clone(),
            srv_net: srv_net.clone(),

            conn_fn: Arc::new(conn_fn),
            pkt_fn: Arc::new(pkt_fn),
            close_fn: Arc::new(close_fn),
        }
    }

    ///
    pub fn on_ll_connect(self: &Arc<Self>, conn: Arc<TcpConn>) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        // 连接计数 + 1
        let old_connection_num = self.connection_num.fetch_add(1, Ordering::Relaxed);
        assert!(old_connection_num < 100000000);

        //
        log::info!(
            "[on_ll_connect][hd={}] connection_limit({}) connection_num: {} -> {}",
            conn.hd,
            self.connection_limit,
            old_connection_num,
            self.connection_num()
        );

        // post 到指定 srv 工作线程中执行
        let conn_fn = self.conn_fn.clone();
        self.srv.run_in_service(Box::new(move || {
            (*conn_fn)(conn);
        }));
    }

    ///
    pub fn on_ll_input(
        self: &Arc<Self>,
        conn: Arc<TcpConn>,
        input_buffer: NetPacketGuard,
        packet_builder: &mut PacketBuilder,
    ) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        packet_builder.build(&conn, input_buffer);
    }

    ///
    pub fn on_ll_disconnect(self: &Arc<Self>, hd: ConnId) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        // post 到指定 srv 工作线程中执行
        let srv = self.srv.clone();
        let close_fn = self.close_fn.clone();
        srv.run_in_service(Box::new(move || {
            (*close_fn)(hd);
        }));

        // 连接计数 - 1
        let old_connection_num = self.connection_num.fetch_sub(1, Ordering::Relaxed);
        assert!(old_connection_num < 100000000);

        //
        log::info!(
            "[on_ll_disconnect][hd={}] connection_limit({}) connection_num: {} -> {}",
            hd,
            self.connection_limit,
            old_connection_num,
            self.connection_num()
        );
    }

    /// Create a tcp server and listen on [ip:port]
    pub fn listen<T, F>(self: &Arc<Self>, srv: &Arc<T>, cb: F)
    where
        T: ServiceRs + 'static,
        F: Fn(ListenerId, SocketAddr) + Send + Sync + 'static,
    {
        //
        self.set_status(ServerStatus::Starting);

        log::info!(
            "tcp server start listen at addr: {} ... status: {}",
            self.addr,
            self.status().to_string()
        );

        //
        let srv = srv.clone();
        let cb = Arc::new(cb);

        let tcp_server = self.clone();
        let listen_fn = move |r| {
            match r {
                Ok((listener_id, sock_addr)) => {
                    // 状态：Running
                    tcp_server.set_status(ServerStatus::Running);

                    // post 到指定 srv 工作线程中执行
                    let cb = cb.clone();
                    srv.run_in_service(Box::new(move || {
                        //
                        (cb)(listener_id, sock_addr);
                    }));
                }
                Err(error) => {
                    //
                    log::error!(
                        "tcp server listen at {:?} failed!!! status:{}!!! error: {}!!!",
                        tcp_server.addr,
                        tcp_server.status().to_string(),
                        error
                    );
                }
            }
        };

        //
        let tcp_server2 = self.clone();
        let accept_fn = move |_listener_id, hd, sock_addr| {
            // make new connection
            tcp_server_make_new_conn(&tcp_server2, hd, sock_addr);
        };

        // start listener
        let listener = Arc::new(Listener::new(
            std::format!("listener({})", self.addr).as_str(),
            listen_fn,
            accept_fn,
            &self.netctrl,
            &self.srv_net,
        ));
        listener.listen_with_tcp(self.addr.as_str());
    }

    ///
    pub fn stop(&self) {
        // TODO:
    }

    ///
    #[inline(always)]
    pub fn connection_num(&self) -> usize {
        self.connection_num.load(Ordering::Relaxed)
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

    ///
    pub fn pkt_fn_clone(&self) -> Arc<dyn Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync> {
        //
        self.pkt_fn.clone()
    }

    ///
    pub fn check_connection_limit(&self) -> bool {
        // check 连接数上限 (0 == self.connection_limit 代表无限制)
        if self.connection_limit > 0 {
            let connection_num = self.connection_num();
            if connection_num >= self.connection_limit {
                //
                log::error!(
                    "**** **** [check_connection_limit()] connection_limit({}/{}) reached!!! **** ****",
                    self.connection_limit,
                    connection_num
                );

                // 已经达到上限
                true
            } else {
                // 未达到上限
                false
            }
        } else {
            // 无需检测上限 == 未达到上限
            false
        }
    }
}
