use atomic::Atomic;
use parking_lot::RwLock;
use std::cell::UnsafeCell;
use std::net::SocketAddr;
use std::sync::Arc;
use uuid::Uuid;

use commlib::with_tls_mut;
use net_packet::NetPacketGuard;
use pinky_swear::PinkySwear;

use crate::service_net::tcp_conn_manager::on_connection_established;
use crate::{ServiceNetRs, ServiceRs};

use super::service_net_impl::create_tcp_client;
use super::tcp_conn_manager::insert_connection;
use super::{ClientStatus, ConnId, PacketType, TcpClient, TcpConn};

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
pub fn insert_tcp_client(srv_net: &ServiceNetRs, cli_id: Uuid, cli: &Arc<TcpClient>) {
    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    with_tls_mut!(G_TCP_CLIENT_STORAGE, g, {
        log::info!("add client: id<{}>", cli_id);
        g.client_table.insert(cli_id, cli.clone());
    });
}

///
#[allow(dead_code)]
pub fn remove_tcp_client(srv_net: &ServiceNetRs, cli_id: Uuid) -> Option<Arc<TcpClient>> {
    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    with_tls_mut!(G_TCP_CLIENT_STORAGE, g, {
        if let Some(cli) = g.client_table.remove(&cli_id) {
            log::info!("remove client: id<{}>", cli_id);
            Some(cli)
        } else {
            log::error!("client: id<{}> not found!!!", cli_id);
            None
        }
    })
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
) -> Option<Arc<TcpClient>>
where
    T: ServiceRs + 'static,
    C: Fn(Arc<TcpConn>) + Send + Sync + 'static,
    P: Fn(Arc<TcpConn>, NetPacketGuard) + Send + Sync + 'static,
    S: Fn(ConnId) + Send + Sync + 'static,
{
    log::info!("({})[connect_to_tcp_server] raddr: {} ...", name, raddr);

    let (promise, pinky) = PinkySwear::<Option<Arc<TcpClient>>>::new();

    // 投递到 srv_net 线程
    let srv_net2 = srv_net.clone();
    let srv2 = srv.clone();
    let name = name.to_owned();
    let raddr = raddr.to_owned();

    let func = move || {
        //
        let cli = Arc::new(create_tcp_client(
            &srv2,
            name.as_str(),
            raddr.as_str(),
            conn_fn,
            pkt_fn,
            close_fn,
            &srv_net2,
        ));
        log::info!(
            "id<{}>({}) start connect to raddr: {} ... ",
            cli.id(),
            cli.name(),
            raddr,
        );

        // insert tcp client
        insert_tcp_client(&srv_net2, cli.id(), &cli);

        //
        let cli2 = cli.clone();
        cli.connect(move |cli, err_opt| {
            if let Some(err) = err_opt {
                log::error!(
                    "id<{}>({}) connect failed!!! error: {}!!!",
                    cli.id(),
                    cli.name(),
                    err
                );

                //
                pinky.swear(None);
            } else {
                // success
                pinky.swear(Some(cli2.clone()));
            }
        });
    };

    srv_net.run_in_service(Box::new(func));

    //
    promise.wait()
}

/// Make new tcp conn with callbacks from tcp client
pub fn tcp_client_make_new_conn(cli: &Arc<TcpClient>, hd: ConnId, sock_addr: SocketAddr) {
    // 运行于 srv_net 线程
    assert!(cli.srv_net().is_in_service_thread());

    //
    let packet_type = Atomic::new(PacketType::Server);

    //
    let cli2 = cli.clone();
    let connection_establish_fn = Box::new(move |conn: Arc<TcpConn>| {
        // 运行于 srv_net 线程
        assert!(conn.srv_net_opt.as_ref().unwrap().is_in_service_thread());
        cli2.on_ll_connect(conn);
    });

    // use packet builder to handle input buffer
    let cli2 = cli.clone();
    let connection_read_fn = Box::new(move |conn: Arc<TcpConn>, input_buffer: NetPacketGuard| {
        // 运行于 srv_net 线程
        assert!(conn.srv_net_opt.as_ref().unwrap().is_in_service_thread());
        cli2.on_ll_input(conn, input_buffer);
    });

    //
    let cli2 = cli.clone();
    let connection_lost_fn = Arc::new(move |hd: ConnId| {
        // 运行于 srv_net 线程
        assert!(cli2.srv_net().is_in_service_thread());
        cli2.on_ll_disconnect(hd);
    });

    //
    let netctrl = cli.netctrl().clone();
    let srv_net = cli.srv_net().clone();

    let conn = Arc::new(TcpConn {
        //
        hd,

        //
        sock_addr,

        //
        packet_type,
        closed: Atomic::new(false),

        //
        netctrl_opt: Some(netctrl.clone()),
        srv_net_opt: Some(srv_net.clone()),

        //
        connection_establish_fn,
        connection_read_fn,
        connection_lost_fn: RwLock::new(connection_lost_fn),
    });

    // add conn
    insert_connection(srv_net.as_ref(), conn.hd, &conn.clone());

    // update inner hd for TcpClient
    {
        cli.set_inner_hd(hd);
    }

    // connection ok
    on_connection_established(conn);
}

///
pub fn tcp_client_check_auto_reconnect(hd: ConnId, cli_id: Uuid, srv_net: &Arc<ServiceNetRs>) {
    // 投递到 srv_net 线程
    let func = move || {
        // close tcp client
        with_tls_mut!(G_TCP_CLIENT_STORAGE, g, {
            if let Some(cli) = g.client_table.get(&cli_id) {
                cli.set_status(ClientStatus::Disconnected);
                log::info!(
                    "[hd={}]({}) check_auto_reconnect ... id<{}> [inner_hd={}]",
                    hd,
                    cli.name(),
                    cli_id,
                    cli.inner_hd(),
                );

                // check auto reconnect
                cli.check_auto_reconnect();
            } else {
                log::error!(
                    "[hd={}] check_auto_reconnect failed!!! id<{}> not found!!!",
                    hd,
                    cli_id,
                );
            }
        });
    };
    srv_net.run_in_service(Box::new(func));
}

///
pub fn tcp_client_reconnect(hd: ConnId, name: &str, cli_id: Uuid, srv_net: &Arc<ServiceNetRs>) {
    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    with_tls_mut!(G_TCP_CLIENT_STORAGE, g, {
        let client_opt = g.client_table.get(&cli_id);
        if let Some(cli) = client_opt {
            if cli.status().is_connected() {
                log::error!(
                    "[hd={}]({}) tcp client reconnect failed!!! id<{}>!!! already connected!!!",
                    hd,
                    name,
                    cli_id,
                );
            } else {
                let name = name.to_owned();
                cli.connect(move |_cli, err_opt| {
                    if let Some(err) = err_opt {
                        log::error!(
                            "[hd={}]({}) tcp client reconnect failed!!! id<{}>!!! error: {}!!!",
                            hd,
                            name,
                            cli_id,
                            err
                        );
                    } else {
                        log::info!(
                            "[hd={}]({}) tcp client reconnect success ... id<{}>.",
                            hd,
                            name,
                            cli_id
                        );
                    }
                });
            }
        } else {
            log::error!(
                "[hd={}]({}) redis client reconnect failed!!! id<{}>!!! client not exist!!!",
                hd,
                name,
                cli_id
            );
        }
    });
}
