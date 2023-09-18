use atomic::Atomic;
use parking_lot::RwLock;
use std::cell::UnsafeCell;
use std::sync::Arc;
use uuid::Uuid;

use message_io::network::Endpoint;

use crate::{ClientStatus, ConnId, PinkySwear, RedisClient, ServiceNetRs, ServiceRs, TcpConn};
use crate::{NetPacketGuard, PacketType};

use crate::service_net::create_redis_client;
use crate::service_net::tcp_conn_manager::insert_connection;
use crate::service_net::ReplyBuilder;

thread_local! {
     static G_REDIS_CLIENT_STORAGE: UnsafeCell<RedisClientStorage> = UnsafeCell::new(RedisClientStorage::new());
}

struct RedisClientStorage {
    /// tcp client table
    client_table: hashbrown::HashMap<uuid::Uuid, Arc<RedisClient>>,
}

impl RedisClientStorage {
    ///
    pub fn new() -> Self {
        Self {
            client_table: hashbrown::HashMap::new(),
        }
    }
}

///
pub fn connect_to_redis<T>(
    srv: &Arc<T>,
    raddr: &str,
    pass: &str,
    dbindex: isize,
    srv_net: &Arc<ServiceNetRs>,
) -> Option<ConnId>
where
    T: ServiceRs + 'static,
{
    log::info!(
        "connect_to_redis: {} ... pass({}) dbindex({})",
        raddr,
        pass,
        dbindex
    );

    let (promise, pinky) = PinkySwear::<Option<ConnId>>::new();

    // 在 srv_net 中运行
    let srv_net2 = srv_net.clone();
    let srv2 = srv.clone();
    let raddr = raddr.to_owned();
    let pass = pass.to_owned();

    let func = move || {
        //
        let cli = Arc::new(create_redis_client(
            &srv2,
            raddr.as_str(),
            pass.as_str(),
            dbindex,
            &srv_net2,
        ));
        log::info!(
            "[connect_to_redis] [hd={}]({}) start connect to {} -- id<{}> ... ",
            cli.inner_hd(),
            cli.name(),
            raddr,
            cli.id(),
        );

        // setup callbacks
        let cli2 = cli.clone();
        let conn_fn = move |conn: Arc<TcpConn>| {
            cli2.on_connect(conn);
        };

        let cli3 = cli.clone();
        let pkt_fn = move |conn: Arc<TcpConn>, pkt: NetPacketGuard| {
            let slice = pkt.peek();
            //cli3.on_receive_reply(conn, slice);
        };

        let cli4 = cli.clone();
        let close_fn = move |hd: ConnId| {
            cli4.on_disconnect(hd);
        };
        cli.setup_callbacks(conn_fn, pkt_fn, close_fn);

        // insert redis client
        with_tls_mut!(G_REDIS_CLIENT_STORAGE, g, {
            log::info!("[connect_to_redis] add client: id<{}>", cli.id());
            g.client_table.insert(cli.id(), cli.clone());
        });

        //
        match cli.connect() {
            Ok(hd) => {
                log::info!("[connect_to_redis][hd={}] client added to service net.", hd);

                //
                pinky.swear(Some(hd));
            }
            Err(err) => {
                log::error!("[connect_to_redis] connect failed!!! error: {}", err);

                //
                pinky.swear(None);
            }
        }
    };
    srv_net.run_in_service(Box::new(func));

    //
    promise.wait()
}

/// Make new tcp conn with callbacks from redis client
pub fn redis_client_make_new_conn(
    cli: &mut RedisClient,
    packet_type: PacketType,
    hd: ConnId,
    endpoint: Endpoint,
) {
    //
    let cli_id = cli.id();

    //
    let netctrl = cli.netctrl().clone();
    let srv_net = cli.srv_net().clone();
    let srv = cli.srv().clone();

    //
    let cli_conn_fn = cli.clone_conn_fn();
    let cli_pkt_fn = cli.clone_pkt_fn();
    let cli_close_fn = cli.clone_close_fn();

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
            redis_client_check_auto_reconnect(&srv_net2, hd, cli_id);
        });

        // use redis reply builder to handle input buffer
        let srv_net3 = srv_net.clone();
        let reply_builder = ReplyBuilder::new();
        let read_fn = Box::new(move |conn: &Arc<TcpConn>, input_buffer: NetPacketGuard| {
            reply_builder.build(srv_net3.as_ref(), conn, input_buffer);
        });

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
            read_fn,
        });

        // add conn
        insert_connection(srv_net.as_ref(), conn.hd, &conn.clone());

        // update inner hd for RedisClient
        with_tls_mut!(G_REDIS_CLIENT_STORAGE, g, {
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
        let f = conn.conn_fn.clone();
        let srv = conn.srv.clone();
        srv.run_in_service(Box::new(move || {
            (f)(conn);
        }));
    };
    cli.srv_net().run_in_service(Box::new(func));
}

///
pub fn redis_client_check_auto_reconnect(srv_net: &ServiceNetRs, hd: ConnId, cli_id: Uuid) {
    // 在 srv_net 中运行
    let func = move || {
        // close tcp client
        with_tls_mut!(G_REDIS_CLIENT_STORAGE, g, {
            if let Some(cli) = g.client_table.get(&cli_id) {
                cli.set_status(ClientStatus::Disconnected);
                log::info!(
                    "[hd={}] close_fn ok -- id<{}> [inner_hd={}]({})",
                    hd,
                    cli_id,
                    cli.inner_hd(),
                    cli.name(),
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
pub fn redis_client_reconnect(srv_net: &ServiceNetRs, hd: ConnId, name: &str, cli_id: Uuid) {
    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    with_tls_mut!(G_REDIS_CLIENT_STORAGE, g, {
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
