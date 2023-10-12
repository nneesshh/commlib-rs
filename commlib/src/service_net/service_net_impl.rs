use atomic::{Atomic, Ordering};
use std::sync::Arc;

use crate::{NodeState, ServiceHandle, ServiceRs};

use super::tcp_server_manager::notify_tcp_server_stop;
use super::MessageIoNetwork;
use super::{ConnId, NetPacketGuard, RedisClient, TcpClient, TcpConn, TcpServer};

static NEXT_CLIENT_ID: Atomic<usize> = Atomic::<usize>::new(1);

/// ServiceNetRs
pub struct ServiceNetRs {
    pub handle: ServiceHandle,

    //
    mi_network: Arc<MessageIoNetwork>,
}

impl ServiceNetRs {
    ///
    pub fn new(id: u64) -> Self {
        Self {
            handle: ServiceHandle::new(id, NodeState::Idle),
            mi_network: Arc::new(MessageIoNetwork::new()),
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

        //
        self.get_handle().join_service();
    }
}

/// Start network event loop over service net
pub fn start_network(srv_net: &Arc<ServiceNetRs>) {
    log::info!("service net start network ...");

    // inner network run in async mode -- loop in a isolate thread
    srv_net.mi_network.start_network_async(srv_net);
}

/// Stop network event loop over service net
pub fn stop_network(srv_net: &Arc<ServiceNetRs>) {
    log::info!("service net stop network ...");

    // inner server stop
    srv_net.mi_network.stop();
}

/// Create tcp server: mi_network is private
pub fn create_tcp_server<T, C, P, S>(
    srv: &Arc<T>,
    addr: &str,
    conn_fn: C,
    pkt_fn: P,
    close_fn: S,
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
        &srv_net.mi_network,
        &srv_net,
    );
    tcp_server
}

/// Create tcp client: mi_network is private
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
        &srv_net.mi_network,
        conn_fn,
        pkt_fn,
        close_fn,
        srv_net,
    )
}

/// Create redis client: mi_network is private
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
        &srv_net.mi_network,
        conn_fn,
        close_fn,
        srv_net,
    )
}
