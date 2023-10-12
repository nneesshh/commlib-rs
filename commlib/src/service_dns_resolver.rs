//!
//! ServiceDnsResolverRs
//!

use std::net::ToSocketAddrs;
use std::sync::Arc;

use crate::{Connector, NodeState, ServiceHandle, ServiceNetRs, ServiceRs};

///
pub struct ServiceDnsResolverRs {
    pub handle: ServiceHandle,
}

impl ServiceDnsResolverRs {
    ///
    pub fn new(id: u64) -> Self {
        Self {
            handle: ServiceHandle::new(id, NodeState::Idle),
        }
    }

    ///
    pub fn resolve(
        self: &Arc<Self>,
        connector: &Arc<Connector>,
        raddr: &str,
        srv_net: &Arc<ServiceNetRs>,
    ) {
        log::info!("resolve raddr: {} ...", raddr);

        // 投递到 service dns resolver
        let srv_dns_resolver = self.clone();
        let srv_net2 = srv_net.clone();
        let connector2 = connector.clone();
        let raddr2 = raddr.to_owned();
        self.run_in_service(Box::new(move || {
            //
            srv_dns_resolver.do_resolve(&connector2, raddr2.as_str(), &srv_net2)
        }));
    }

    ///
    fn do_resolve(
        self: &Arc<Self>,
        connector: &Arc<Connector>,
        raddr: &str,
        srv_net: &Arc<ServiceNetRs>,
    ) {
        //
        match raddr.to_socket_addrs() {
            Ok(addr_iter) => {
                // 遍历找到 ipv4
                let addrs = addr_iter.collect::<Vec<_>>();
                for addr in addrs {
                    if addr.is_ipv4() {
                        // 投递到 srv_net 线程
                        let connector2 = connector.clone();
                        let raddr2 = raddr.to_owned();
                        srv_net.run_in_service(Box::new(move || {
                            log::info!("try_resolve raddr: {} success", raddr2);
                            connector2.on_sock_addr_ready(addr);
                        }));
                        return;
                    }
                }

                // 投递到 srv_net 线程
                let connector2 = connector.clone();
                srv_net.run_in_service(Box::new(move || {
                    // 如果没有找到 ipv4 地址，则失败
                    (connector2.ready_cb)(Err("NoIpV4Addr".to_owned()));
                }))
            }
            Err(error) => {
                // 投递到 srv_net 线程
                let connector2 = connector.clone();
                srv_net.run_in_service(Box::new(move || {
                    (connector2.ready_cb)(Err(error.to_string()));
                }))
            }
        }
    }
}

impl ServiceRs for ServiceDnsResolverRs {
    /// 获取 service name
    #[inline(always)]
    fn name(&self) -> &str {
        "service_dns_resolver"
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
    fn run_in_service(&self, cb: Box<dyn FnOnce() + Send + 'static>) {
        self.get_handle().run_in_service(cb);
    }

    /// 当前代码是否运行于 service 线程中
    #[inline(always)]
    fn is_in_service_thread(&self) -> bool {
        self.get_handle().is_in_service_thread()
    }

    /// 等待线程结束
    fn join(&self) {
        self.get_handle().join_service();
    }
}
