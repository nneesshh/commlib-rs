use super::{CmdId, ConnId, ServerCallbacks, TcpConn, TcpServer};
use crate::commlib_service::{NodeState, ServiceHandle, ServiceRs};
use parking_lot::RwLock;

/// ServiceNetRs
pub struct ServiceNetRs {
    pub handle: RwLock<ServiceHandle>,

    pub conn_table: RwLock<hashbrown::HashMap<ConnId, TcpConn>>, // TODO: remove lock?
    pub tcp_server_opt: Option<TcpServer>,
}

impl ServiceNetRs {
    ///
    pub fn new(id: u64) -> ServiceNetRs {
        Self {
            handle: RwLock::new(ServiceHandle::new(id, NodeState::Idle)),

            conn_table: RwLock::new(hashbrown::HashMap::with_capacity(4096)),
            tcp_server_opt: Some(TcpServer::new()),
        }
    }

    /// Send over tcp conn
    pub fn send(&self, hd: ConnId, data: &[u8]) {
        let conn_table = self.conn_table.read();
        if let Some(tcp_conn) = conn_table.get(&hd) {
            tcp_conn.send(data);
        }
    }

    /// Listen on [ip:port] and start tcp server
    pub fn listen(
        &'static self,
        ip: String,
        port: u16,
        callbacks: ServerCallbacks,
        srv: &'static dyn ServiceRs,
    ) {
        //
        let tcp_server = self.tcp_server_opt.as_ref().unwrap();

        //
        let cb = move || {
            // tcp server start
            tcp_server.listen(ip, port, callbacks, self);

            // tcp server start
            tcp_server.start(self);
        };
        self.run_in_service(Box::new(cb));
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

    /// Init in-service
    fn init(&self) -> bool {
        true
    }

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
        //
        if let Some(tcp_server) = self.tcp_server_opt.as_ref() {
            tcp_server.stop();
        }

        //
        {
            let mut handle_mut = self.get_handle().write();
            handle_mut.join_service();
        }
    }
}
