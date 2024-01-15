//!
//! DnsResolverRs
//!

use std::net::ToSocketAddrs;
use std::sync::Arc;

use commlib::G_THREAD_POOL;

use crate::service_net::connector::Connector;
use crate::{ServiceNetRs, ServiceRs};

pub fn dns_resolve(connector: &Arc<Connector>, raddr: &str, srv_net: &Arc<ServiceNetRs>) {
    log::info!("resolve raddr: {} ...", raddr);

    let connector2 = connector.clone();
    let raddr2 = raddr.to_owned();
    let srv_net2 = srv_net.clone();

    // post 到线程池
    G_THREAD_POOL.execute_rr(move || {
        //
        do_dns_resolve(&connector2, raddr2.as_str(), &srv_net2);
    });
}

///
pub fn do_dns_resolve(connector: &Arc<Connector>, raddr: &str, srv_net: &Arc<ServiceNetRs>) {
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
                        connector2.on_addr_ready(addr);
                    }));
                    return;
                }
            }

            // 投递到 srv_net 线程
            let connector2 = connector.clone();
            srv_net.run_in_service(Box::new(move || {
                // 如果没有找到 ipv4 地址，则失败
                (connector2.connect_fn)(Err("NoIpV4Addr".to_owned()));
            }))
        }
        Err(error) => {
            // 投递到 srv_net 线程
            let connector2 = connector.clone();
            srv_net.run_in_service(Box::new(move || {
                (connector2.connect_fn)(Err(error.to_string()));
            }))
        }
    }
}
