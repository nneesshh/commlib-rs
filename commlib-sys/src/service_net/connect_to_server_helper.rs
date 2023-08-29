use std::sync::Arc;

use crate::{ServiceNetRs, ServiceRs};

use super::create_tcp_client;
use super::{ConnId, NetPacketGuard};

///
pub fn connect_to_tcp_server<T, C, P, S>(
    srv: &Arc<T>,
    name: &str,
    raddr: &str,
    conn_fn: C,
    pkt_fn: P,
    close_fn: S,
    srv_net: &Arc<ServiceNetRs>,
) -> Option<ConnId>
where
    T: ServiceRs + 'static,
    C: Fn(ConnId) + Send + Sync + 'static,
    P: Fn(ConnId, NetPacketGuard) + Send + Sync + 'static,
    S: Fn(ConnId) + Send + Sync + 'static,
{
    //
    let cli = create_tcp_client(srv, name, raddr, conn_fn, pkt_fn, close_fn, srv_net);
    log::info!(
        "[connect_to_tcp_server] start connect to {} -- id<{}> ... ",
        cli.id,
        raddr
    );

    //
    match cli.connect() {
        Ok(hd) => {
            log::info!(
                "[connect_to_tcp_server][hd={}] client added to service net.",
                hd
            );

            //
            Some(hd)
        }
        Err(err) => {
            log::error!("[connect_to_tcp_server] connect failed!!! error: {}", err);

            //
            None
        }
    }
}
