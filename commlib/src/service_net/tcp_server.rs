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
use std::cell::UnsafeCell;
use std::net::SocketAddr;
use std::sync::Arc;
use thread_local::ThreadLocal;

use crate::{ServiceNetRs, ServiceRs};

use super::low_level_network::MessageIoNetwork;
use super::{ConnId, NetPacketGuard, PacketBuilder, ServerStatus, TcpConn, TcpListenerId};

///
pub struct TcpServer {
    //
    status: Atomic<ServerStatus>,

    connection_limit: Atomic<usize>, // TODO
    connection_num: Atomic<usize>,   // TODO

    //
    pub addr: String,
    pub listener_id: TcpListenerId,
    pub listen_fn: Arc<dyn Fn(SocketAddr, ServerStatus) + Send + Sync>,

    //
    srv: Arc<dyn ServiceRs>,
    netctrl: Arc<MessageIoNetwork>,
    srv_net: Arc<ServiceNetRs>,

    //
    conn_fn: Arc<dyn Fn(Arc<TcpConn>) + Send + Sync>,
    pkt_fn: Arc<dyn Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync>,
    close_fn: Arc<dyn Fn(ConnId) + Send + Sync>,

    //
    tls_pkt_builder: ThreadLocal<UnsafeCell<PacketBuilder>>,
}

impl TcpServer {
    ///
    pub fn new<T, C, P, S>(
        srv: &Arc<T>,
        addr: &str,
        conn_fn: C,
        pkt_fn: P,
        close_fn: S,
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

            connection_limit: Atomic::new(0_usize),
            connection_num: Atomic::new(0_usize),

            addr: addr.to_owned(),
            listener_id: TcpListenerId::from(0),
            listen_fn: Arc::new(|_sock_addr, _status| {}),

            srv: srv.clone(),
            netctrl: netctrl.clone(),
            srv_net: srv_net.clone(),

            conn_fn: Arc::new(conn_fn),
            pkt_fn: Arc::new(pkt_fn),
            close_fn: Arc::new(close_fn),

            tls_pkt_builder: ThreadLocal::new(),
        }
    }

    ///
    pub fn on_ll_connect(self: &Arc<Self>, conn: Arc<TcpConn>) {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        // post 到指定 srv 工作线程中执行
        let conn_fn = self.conn_fn.clone();
        self.srv.run_in_service(Box::new(move || {
            (*conn_fn)(conn);
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

        // post 到指定 srv 工作线程中执行
        let srv = self.srv.clone();
        let close_fn = self.close_fn.clone();
        srv.run_in_service(Box::new(move || {
            (*close_fn)(hd);
        }));
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
        let netctrl = self.netctrl().clone();
        if !netctrl.listen(self) {
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
    pub fn stop(&self) {
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

    ////////////////////////////////////////////////////////////////

    #[inline(always)]
    fn get_packet_builder<'a>(self: &'a Arc<Self>) -> &'a mut PacketBuilder {
        // 运行于 srv_net 线程
        assert!(self.srv_net.is_in_service_thread());

        //
        let builder = self.tls_pkt_builder.get_or(|| {
            let srv = self.srv.clone();
            let pkt_fn = self.pkt_fn.clone();

            let build_cb = move |conn: Arc<TcpConn>, pkt: NetPacketGuard| {
                // 运行于 srv_net 线程
                assert!(conn.srv_net.is_in_service_thread());

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
