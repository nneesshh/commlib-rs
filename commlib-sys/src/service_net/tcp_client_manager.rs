use atomic::Atomic;
use parking_lot::RwLock;
use std::cell::UnsafeCell;
use std::sync::Arc;
use uuid::Uuid;

use message_io::network::Endpoint;

use crate::{PinkySwear, ServiceNetRs, ServiceRs};

use super::create_tcp_client;
use super::tcp_conn_manager::{insert_connection, run_conn_fn};
use super::{ClientStatus, ConnId, NetPacketGuard, PacketReceiver, PacketType, TcpClient, TcpConn};

thread_local! {
     static G_TCP_CLIENT_STORAGE: UnsafeCell<TcpClientStorage> = UnsafeCell::new(TcpClientStorage::new());
}

struct TcpClientStorage {
    /// tcp client table
    client_table: hashbrown::HashMap<uuid::Uuid, Arc<TcpClient>>,
}

impl TcpClientStorage {
    ///
    pub fn new() -> Self {
        Self {
            client_table: hashbrown::HashMap::new(),
        }
    }
}

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
    C: Fn(Arc<TcpConn>) + Send + Sync + 'static,
    P: Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync + 'static,
    S: Fn(ConnId) + Send + Sync + 'static,
{
    log::info!("connect_to_tcp_server: {} -- {}...", name, raddr);

    let (promise, pinky) = PinkySwear::<Option<ConnId>>::new();

    // 在 srv_net 中运行
    let srv_net2 = srv_net.clone();
    let srv2 = srv.clone();
    let name = name.to_owned();
    let raddr = raddr.to_owned();
    let func = move || {
        //
        let cli = create_tcp_client(
            &srv2,
            name.as_str(),
            raddr.as_str(),
            conn_fn,
            pkt_fn,
            close_fn,
            &srv_net2,
        );
        log::info!(
            "[connect_to_tcp_server] [hd={}]({}) start connect to {} -- id<{}> ... ",
            cli.inner_hd(),
            cli.name,
            raddr,
            cli.id,
        );

        // insert tcp client
        with_tls_mut!(G_TCP_CLIENT_STORAGE, g, {
            log::info!("add client: id<{}>", cli.id);
            g.client_table.insert(cli.id, cli.clone());
        });

        //
        match cli.connect() {
            Ok(hd) => {
                log::info!(
                    "[connect_to_tcp_server][hd={}] client added to service net.",
                    hd
                );

                //
                pinky.swear(Some(hd));
            }
            Err(err) => {
                log::error!("[connect_to_tcp_server] connect failed!!! error: {}", err);

                //
                pinky.swear(None);
            }
        }
    };
    srv_net.run_in_service(Box::new(func));

    //
    promise.wait()
}

/// Make new tcp conn with callbacks from tcp client
pub fn tcp_client_make_new_conn(
    cli: &mut TcpClient,
    packet_type: PacketType,
    hd: ConnId,
    endpoint: Endpoint,
) {
    //
    let cli_id = cli.id.clone();
    let netctrl = cli.mi_network.node_handler.clone();

    //
    let cli_conn_fn = cli.conn_fn.clone();
    let cli_pkt_fn = cli.pkt_fn.clone();
    let cli_close_fn = cli.close_fn.clone();

    let srv_net = cli.srv_net.clone();
    let srv = cli.srv.clone();

    // insert tcp conn in srv net(同一线程便于观察 conn 生命周期)
    let func = move || {
        let conn_fn = Arc::new(move |conn| {
            (*cli_conn_fn)(conn);
        });
        let pkt_fn = Arc::new(move |conn, pkt| {
            (*cli_pkt_fn)(conn, pkt);
        });

        let srv_net2 = srv_net.clone();
        let close_fn = Arc::new(move |hd| {
            // close fn
            (*cli_close_fn)(hd);

            // check auto reconnect
            tcp_client_check_auto_reconnect(&srv_net2, hd, cli_id);
        });

        let srv_net3 = srv_net.clone();
        let conn = Arc::new(TcpConn {
            //
            hd,

            //
            endpoint,
            netctrl: netctrl.clone(),

            //
            packet_type: Atomic::new(packet_type),
            closed: Atomic::new(false),

            //
            srv: srv.clone(),
            srv_net: srv_net.clone(),

            //
            conn_fn,
            pkt_fn,
            close_fn: RwLock::new(close_fn),

            //
            pkt_receiver: PacketReceiver::new(),
        });

        // add conn
        insert_connection(srv_net3.as_ref(), conn.hd, &conn.clone());

        // update inner hd for TcpClient
        with_tls_mut!(G_TCP_CLIENT_STORAGE, g, {
            if let Some(cli) = g.client_table.get(&cli_id) {
                cli.set_inner_hd(hd);
            } else {
                log::error!(
                    "[hd={}] update inner hd failed!!! id<{}> not found!!!",
                    hd,
                    cli_id,
                );
            }
        });

        // trigger conn_fn
        run_conn_fn(&conn);
    };
    cli.srv_net.run_in_service(Box::new(func));
}

///
pub fn tcp_client_check_auto_reconnect(srv_net: &ServiceNetRs, hd: ConnId, cli_id: Uuid) {
    // 在 srv_net 中运行
    let func = move || {
        // close tcp client
        with_tls_mut!(G_TCP_CLIENT_STORAGE, g, {
            if let Some(cli) = g.client_table.get(&cli_id) {
                cli.set_status(ClientStatus::Disconnected);
                log::info!(
                    "[hd={}] close_fn ok -- id<{}> [inner_hd={}]({})",
                    hd,
                    cli_id,
                    cli.inner_hd(),
                    cli.name,
                );

                // check auto reconnect
                cli.check_auto_reconnect();
            } else {
                log::error!("[hd={}] close_fn failed!!! id<{}> not found!!!", hd, cli_id,);
            }
        });
    };
    srv_net.run_in_service(Box::new(func));
}

///
pub fn tcp_client_reconnect(srv_net: &ServiceNetRs, hd: ConnId, name: String, cli_id: Uuid) {
    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    with_tls_mut!(G_TCP_CLIENT_STORAGE, g, {
        let client_opt = g.client_table.get(&cli_id);
        if let Some(cli) = client_opt {
            if cli.status().is_connected() {
                log::error!(
                    "[hd={}]({}) reconnect failed -- id<{}>!!! already connected!!!",
                    hd,
                    name,
                    cli_id
                );
            } else {
                match cli.connect() {
                    Ok(hd) => {
                        log::info!("[hd={}]({}) reconnect success -- id<{}>.", hd, name, cli_id);
                    }
                    Err(err) => {
                        log::error!(
                            "[hd={}]({}) reconnect failed -- id<{}>!!! error: {}!!!",
                            hd,
                            name,
                            cli_id,
                            err
                        );
                    }
                }
            }
        } else {
            log::error!(
                "[hd={}]({}) reconnect failed -- id<{}>!!! client not exist!!!",
                hd,
                name,
                cli_id
            );
        }
    });
}
