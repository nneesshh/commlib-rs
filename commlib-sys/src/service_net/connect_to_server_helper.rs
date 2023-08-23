use std::sync::Arc;

use crate::{ServiceNetRs, ServiceRs};

use super::{ConnId, NetPacketGuard, TcpClient};

///
pub fn create_tcp_client<T>(srv: &Arc<T>, name: &str, raddr: &str) -> TcpClient
where
    T: ServiceRs + 'static,
{
    TcpClient::new(name, raddr, srv)
}

///
pub fn connect_to_tcp_server<T, C, P, S>(
    srv: &Arc<T>,
    name: &str,
    raddr: &str,
    conn_fn: C,
    pkt_fn: P,
    stopped_cb: S,
    srv_net: &Arc<ServiceNetRs>,
) -> ConnId
where
    T: ServiceRs + 'static,
    C: Fn(ConnId) + Send + Sync + 'static,
    P: Fn(ConnId, NetPacketGuard) + Send + Sync + 'static,
    S: Fn(ConnId) + Send + Sync + 'static,
{
    let mut cli = TcpClient::new(name, raddr, srv);
    let hd = cli.id;

    //
    cli.set_connection_callback(conn_fn);
    cli.set_message_callback(pkt_fn);
    cli.set_close_callback(stopped_cb);

    //
    cli.connect(srv_net);

    // add client to srv_net
    {
        let mut client_table_mut = srv_net.client_table.write();
        client_table_mut.insert(hd, cli);

        log::info!("[connect_to_tcp_server][hd={:?}] client added", hd);
    }

    //
    hd
}
