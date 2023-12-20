use atomic::{Atomic, Ordering};
use std::sync::Arc;

use net_packet::NetPacketGuard;

use crate::{NodeState, ServiceHandle, ServiceRs};

use super::http_server::http_server_manager::notify_http_server_stop;
use super::http_server::HttpServer;
use super::http_server::ResponseResult;
use super::low_level_network::MessageIoNetwork;
use super::tcp_server::tcp_server_manager::notify_tcp_server_stop;
#[cfg(feature = "websocket")]
use super::ws_server::ws_server_manager::notify_ws_server_stop;
#[cfg(feature = "websocket")]
use super::WsServer;
use super::{ConnId, RedisClient, TcpClient, TcpConn, TcpServer};

static NEXT_CLIENT_ID: Atomic<usize> = Atomic::<usize>::new(1);

/// ServiceNetRs
pub struct ServiceNetRs {
    pub handle: ServiceHandle,

    //
    netctrl: Arc<MessageIoNetwork>,
}

impl ServiceNetRs {
    ///
    pub fn new(id: u64) -> Self {
        Self {
            handle: ServiceHandle::new(id, NodeState::Idle),
            netctrl: Arc::new(MessageIoNetwork::new()),
        }
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

    /// update
    #[inline(always)]
    fn update(&self) {}

    /// 在 service 线程中执行回调任务
    #[inline(always)]
    fn run_in_service(&self, cb: Box<dyn FnOnce() + Send>) {
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
        notify_tcp_server_stop(self);

        // notify http server to stop
        notify_http_server_stop(self);

        // notify ws server to stop
        #[cfg(feature = "websocket")]
        notify_ws_server_stop(self);

        //
        self.get_handle().join_service();
    }
}

/// Start network event loop over service net
pub fn start_network(srv_net: &Arc<ServiceNetRs>) {
    log::info!("service net start network ...");

    // inner network run in async mode -- loop in a isolate thread
    srv_net.netctrl.start_network_async(srv_net);
}

/// Stop network event loop over service net
pub fn stop_network(srv_net: &Arc<ServiceNetRs>) {
    log::info!("service net stop network ...");

    // inner server stop
    srv_net.netctrl.stop();
}

/// Create tcp server: netctrl is private
#[allow(dead_code)]
pub fn create_tcp_server<T, C, P, S>(
    srv: &Arc<T>,
    addr: &str,
    conn_fn: C,
    pkt_fn: P,
    close_fn: S,
    connection_limit: usize,
    srv_net: &Arc<ServiceNetRs>,
) -> TcpServer
where
    T: ServiceRs + 'static,
    C: Fn(Arc<TcpConn>) + Send + Sync + 'static,
    P: Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync + 'static,
    S: Fn(ConnId) + Send + Sync + 'static,
{
    let tcp_server = TcpServer::new(
        &srv,
        addr,
        conn_fn,
        pkt_fn,
        close_fn,
        connection_limit,
        &srv_net.netctrl,
        &srv_net,
    );
    tcp_server
}

/// Create http server: netctrl is private
#[allow(dead_code)]
pub fn create_http_server<C, R, S>(
    addr: &str,
    conn_fn: C,
    request_fn: R,
    close_fn: S,
    connection_limit: usize,
    srv_net: &Arc<ServiceNetRs>,
) -> HttpServer
where
    C: Fn(Arc<TcpConn>) + Send + Sync + 'static,
    R: Fn(Arc<TcpConn>, http::Request<Vec<u8>>, http::response::Builder) -> ResponseResult
        + Send
        + Sync
        + 'static,
    S: Fn(ConnId) + Send + Sync + 'static,
{
    let http_server = HttpServer::new(
        addr,
        conn_fn,
        request_fn,
        close_fn,
        connection_limit,
        &srv_net.netctrl,
        &srv_net,
    );
    http_server
}

/// Create websocket server: netctrl is private
#[allow(dead_code)]
#[cfg(feature = "websocket")]
pub fn create_websocket_server<T, C, P, S>(
    srv: &Arc<T>,
    addr: &str,
    conn_fn: C,
    pkt_fn: P,
    close_fn: S,
    connection_limit: usize,
    srv_net: &Arc<ServiceNetRs>,
) -> WsServer
where
    T: ServiceRs + 'static,
    C: Fn(Arc<TcpConn>) + Send + Sync + 'static,
    P: Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync + 'static,
    S: Fn(ConnId) + Send + Sync + 'static,
{
    let ws_server = WsServer::new(
        &srv,
        addr,
        conn_fn,
        pkt_fn,
        close_fn,
        connection_limit,
        &srv_net.netctrl,
        &srv_net,
    );
    ws_server
}

/// Create tcp client: netctrl is private
#[allow(dead_code)]
pub fn create_tcp_client<T, C, P, S>(
    srv: &Arc<T>,
    name: &str,
    raddr: &str,
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
    TcpClient::new(
        srv,
        name,
        raddr,
        &srv_net.netctrl,
        conn_fn,
        pkt_fn,
        close_fn,
        srv_net,
    )
}

/// Create redis client: netctrl is private
#[allow(dead_code)]
pub fn create_redis_client(
    srv: &Arc<dyn ServiceRs>,
    raddr: &str,
    pass: &str,
    dbindex: isize,
    srv_net: &Arc<ServiceNetRs>,
) -> RedisClient {
    let next_id = NEXT_CLIENT_ID.fetch_add(1, Ordering::Relaxed);
    let name = std::format!("redis{}", next_id);

    let conn_fn = |_1| {};
    let close_fn = |_1| {};

    RedisClient::new(
        srv,
        name.as_str(),
        raddr,
        pass,
        dbindex,
        &srv_net.netctrl,
        conn_fn,
        close_fn,
        srv_net,
    )
}
