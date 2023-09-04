use parking_lot::RwLock;
use std::sync::Arc;

use crate::{Clock, NodeState, PinkySwear, ServiceHandle, ServiceRs};

use super::MessageIoNetwork;
use super::{
    packet_receiver::PacketResult, ConnId, EncryptData, NetPacketGuard, TcpClient, TcpConn,
    TcpListenerId, TcpServer,
};

/// ServiceNetRs
pub struct ServiceNetRs {
    pub handle: ServiceHandle,

    pub client_table: RwLock<hashbrown::HashMap<uuid::Uuid, Arc<TcpClient>>>, // TODO: remove lock?
    pub conn_table: RwLock<hashbrown::HashMap<ConnId, Arc<TcpConn>>>,         // TODO: remove lock?

    pub tcp_server_vec: RwLock<Vec<TcpServer>>, // TODO: remove lock?

    // 每条连接的包序号和密钥
    hd_encrypt_table: RwLock<hashbrown::HashMap<ConnId, Arc<EncryptData>>>, // TODO: remove lock?

    //
    inner_network: Arc<MessageIoNetwork>,
}

impl ServiceNetRs {
    ///
    pub fn new(id: u64) -> ServiceNetRs {
        Self {
            handle: ServiceHandle::new(id, NodeState::Idle),

            client_table: RwLock::new(hashbrown::HashMap::new()),

            conn_table: RwLock::new(hashbrown::HashMap::with_capacity(4096)),
            tcp_server_vec: RwLock::new(Vec::new()),

            hd_encrypt_table: RwLock::new(hashbrown::HashMap::with_capacity(4096)),

            //
            inner_network: Arc::new(MessageIoNetwork::new()),
        }
    }

    ///
    #[inline(always)]
    pub fn get_conn(&self, hd: ConnId) -> Option<Arc<TcpConn>> {
        let conn_table = self.conn_table.read();
        if let Some(conn) = conn_table.get(&hd) {
            Some(conn.clone())
        } else {
            log::error!("[hd={}] get_conn failed -- hd not found!!!", hd);
            None
        }
    }

    /// Add conn
    #[inline(always)]
    pub fn insert_conn(&self, hd: ConnId, conn: &Arc<TcpConn>) {
        let mut conn_table_mut = self.conn_table.write();
        log::info!("[hd={}] ++++++++ service net insert_conn", hd);
        conn_table_mut.insert(hd, conn.clone());
    }

    /// Remove conn
    #[inline(always)]
    pub fn remove_conn(&self, hd: ConnId) -> Option<Arc<TcpConn>> {
        let mut conn_table_mut = self.conn_table.write();
        log::info!("[hd={}] -------- service net remove_conn", hd);
        conn_table_mut.remove(&hd)
    }

    ///
    #[inline(always)]
    pub fn get_client(&self, id: &uuid::Uuid) -> Option<Arc<TcpClient>> {
        let client_table = self.client_table.read();
        if let Some(cli) = client_table.get(id) {
            Some(cli.clone())
        } else {
            log::error!("get_client failed -- id={} not found!!!", id);
            None
        }
    }

    /// Add client
    #[inline(always)]
    pub fn insert_client(&self, id: &uuid::Uuid, cli: &Arc<TcpClient>) {
        let mut client_table_mut = self.client_table.write();
        log::info!("service net insert_client id={},", id);
        client_table_mut.insert(id.clone(), cli.clone());
    }

    /// Remove client
    #[inline(always)]
    pub fn remove_client(&self, id: &uuid::Uuid) -> Option<Arc<TcpClient>> {
        let mut client_table_mut = self.client_table.write();
        log::info!("service net remove_client id={},", id);
        client_table_mut.remove(id)
    }

    /// Send over tcp conn
    #[inline(always)]
    pub fn send(&self, hd: ConnId, data: &[u8]) {
        let conn_table = self.conn_table.read();
        if let Some(conn) = conn_table.get(&hd) {
            conn.send(data);
        }
    }

    ///
    pub fn set_timeout<F>(&self, delay: u64, f: F)
    where
        F: FnMut() + Send + Sync + 'static,
    {
        Clock::set_timeout(self, delay, f);
    }
}

impl ServiceRs for ServiceNetRs {
    /// 获取 service nmae
    #[inline(always)]
    fn name(&self) -> &str {
        "service_net"
    }

    /// 获取 service 句柄
    #[inline(always)]
    fn get_handle(&self) -> &ServiceHandle {
        &self.handle
    }

    /// 配置 service
    fn conf(&self) {}

    /// 在 service 线程中执行回调任务
    #[inline(always)]
    fn run_in_service(&self, cb: Box<dyn FnOnce() + Send + Sync>) {
        self.get_handle().run_in_service(cb);
    }

    /// 当前代码是否运行于 service 线程中
    #[inline(always)]
    fn is_in_service_thread(&self) -> bool {
        self.get_handle().is_in_service_thread()
    }

    /// 等待线程结束
    fn join(&self) {
        // notify tcp server to stop
        {
            let mut tcp_server_vec_mut = self.tcp_server_vec.write();
            for tcp_server in &mut (*tcp_server_vec_mut) {
                tcp_server.stop();
            }
        }

        //
        self.get_handle().join_service();
    }
}

/// Start network event loop over service net
pub fn start_network(srv_net: &Arc<ServiceNetRs>) {
    log::info!("service net start network ...");

    // inner network run in async mode -- loop in a isolate thread
    srv_net.inner_network.start_network_async(srv_net);
}

/// Stop network event loop over service net
pub fn stop_network(srv_net: &Arc<ServiceNetRs>) {
    log::info!("service net stop network ...");

    // inner server stop
    srv_net.inner_network.stop();
}

/// Listen on [ip:port] over service net
pub fn listen_tcp_addr<T, C, P, S>(
    srv: &Arc<T>,
    ip: String,
    port: u16,
    conn_fn: C,
    pkt_fn: P,
    close_fn: S,
    srv_net: &Arc<ServiceNetRs>,
) -> TcpListenerId
where
    T: ServiceRs + 'static,
    C: Fn(ConnId) + Send + Sync + 'static,
    P: Fn(ConnId, NetPacketGuard) + Send + Sync + 'static,
    S: Fn(ConnId) + Send + Sync + 'static,
{
    log::info!("service net listen {}:{}...", ip, port);

    let (promise, pinky) = PinkySwear::<TcpListenerId>::new();

    //
    let srv_net2 = srv_net.clone();
    let srv2 = srv.clone();
    let cb = move || {
        //
        let addr = std::format!("{}:{}", ip, port);
        let mut tcp_server =
            TcpServer::new(&srv2, addr.as_str(), &srv_net2.inner_network, &srv_net2);

        //
        tcp_server.set_connection_callback(conn_fn);
        tcp_server.set_message_callback(pkt_fn);
        tcp_server.set_close_callback(close_fn);

        // listen
        tcp_server.listen();

        // add tcp server to serivce net
        {
            let mut tcp_server_vec_mut = srv_net2.tcp_server_vec.write();
            (*tcp_server_vec_mut).push(tcp_server);
        }

        // pinky for listener_id
        {
            let tcp_server_vec = srv_net2.tcp_server_vec.read();
            let tcp_server = tcp_server_vec.last().unwrap();
            pinky.swear(tcp_server.listener_id);
        }
    };
    srv_net.run_in_service(Box::new(cb));

    //
    promise.wait()
}

/// Create tcp client
pub fn create_tcp_client<T, C, P, S>(
    srv: &Arc<T>,
    name: &str,
    raddr: &str,
    conn_fn: C,
    pkt_fn: P,
    close_fn: S,
    srv_net: &Arc<ServiceNetRs>,
) -> Arc<TcpClient>
where
    T: ServiceRs + 'static,
    C: Fn(ConnId) + Send + Sync + 'static,
    P: Fn(ConnId, NetPacketGuard) + Send + Sync + 'static,
    S: Fn(ConnId) + Send + Sync + 'static,
{
    let cli = Arc::new(TcpClient::new(
        srv,
        name,
        raddr,
        &srv_net.inner_network,
        conn_fn,
        pkt_fn,
        close_fn,
        srv_net,
    ));

    // add client to srv_net
    srv_net.insert_client(&cli.id, &cli);

    //
    cli
}

/// 处理 message 事件 （在 srv_net 中运行）
pub fn handle_message_event(
    srv_net: &ServiceNetRs,
    conn: &Arc<TcpConn>,
    buffer_pkt: NetPacketGuard,
) {
    assert!(srv_net.is_in_service_thread());

    // conn 处理 input
    match conn.handle_read(buffer_pkt) {
        PacketResult::Ready(pkt_list) => {
            // pkt trigger pkt_fn
            for pkt in pkt_list {
                conn.run_pkt_fn(pkt);
            }
        }

        PacketResult::Abort(err) => {
            log::error!("[on_message_cb] handle_read failed!!! error: {}", err);

            // low level close
            conn.close();

            // handle close conn event
            handle_close_conn_event(srv_net, &conn);
        }
    }
}

/// 处理连接关闭事件 （在 srv_net 中运行）
pub fn handle_close_conn_event(srv_net: &ServiceNetRs, conn: &Arc<TcpConn>) {
    assert!(srv_net.is_in_service_thread());

    // remove conn always
    srv_net.remove_conn(conn.hd);

    // trigger close_fn
    conn.run_close_fn();
}
