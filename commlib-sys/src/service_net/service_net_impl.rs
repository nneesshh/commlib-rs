use parking_lot::RwLock;
use std::sync::Arc;

use crate::{NodeState, Pinky, PinkySwear, ServiceHandle, ServiceRs, TcpListenerId};

use super::{start_message_io_network_async, MessageIoNetwork};
use super::{ConnId, NetPacketGuard, TcpClient, TcpConn, TcpServer};

/// ServiceNetRs
pub struct ServiceNetRs {
    pub handle: RwLock<ServiceHandle>,

    pub client_table: RwLock<hashbrown::HashMap<ConnId, TcpClient>>, // TODO: remove lock?

    pub conn_table: RwLock<hashbrown::HashMap<ConnId, TcpConn>>, // TODO: remove lock?
    pub tcp_server_vec: RwLock<Vec<TcpServer>>,                  // TODO: remove lock?

    //
    inner_network: Arc<MessageIoNetwork>,
}

impl ServiceNetRs {
    ///
    pub fn new(id: u64) -> ServiceNetRs {
        Self {
            handle: RwLock::new(ServiceHandle::new(id, NodeState::Idle)),

            client_table: RwLock::new(hashbrown::HashMap::new()),

            conn_table: RwLock::new(hashbrown::HashMap::with_capacity(4096)),
            tcp_server_vec: RwLock::new(Vec::new()),

            //
            inner_network: Arc::new(MessageIoNetwork::new()),
        }
    }

    /// Send over tcp conn
    pub fn send(&self, hd: ConnId, data: &[u8]) {
        let conn_table = self.conn_table.read();
        if let Some(conn) = conn_table.get(&hd) {
            conn.send(data);
        }
    }
}

impl ServiceRs for ServiceNetRs {
    /// 获取 service nmae
    fn name(&self) -> &str {
        "service_net"
    }

    /// 获取 service 句柄
    fn get_handle(&self) -> &RwLock<ServiceHandle> {
        &self.handle
    }

    /// 配置 service
    fn conf(&self) {}

    /// 在 service 线程中执行回调任务
    fn run_in_service(&self, cb: Box<dyn FnOnce() + Send + Sync>) {
        let handle = self.get_handle().read();
        handle.run_in_service(cb);
    }

    /// 当前代码是否运行于 service 线程中
    fn is_in_service_thread(&self) -> bool {
        let handle = self.get_handle().read();
        handle.is_in_service_thread()
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
        {
            let mut handle_mut = self.get_handle().write();
            handle_mut.join_service();
        }
    }
}

/// Start network event loop over service net
pub fn start_network(srv_net: &Arc<ServiceNetRs>) {
    log::info!("service net start network ...");

    // inner network run in async mode -- loop in a isolate thread
    start_message_io_network_async(&srv_net.inner_network, srv_net);
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
    stopped_cb: S,
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
        tcp_server.set_close_callback(stopped_cb);

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
pub fn create_tcp_client<T>(
    srv: &Arc<T>,
    name: &str,
    raddr: &str,
    srv_net: &Arc<ServiceNetRs>,
) -> TcpClient
where
    T: ServiceRs + 'static,
{
    TcpClient::new(srv, name, raddr, &srv_net.inner_network, srv_net)
}
