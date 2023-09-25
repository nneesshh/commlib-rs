use atomic::Atomic;
use parking_lot::RwLock;
use std::cell::UnsafeCell;
use std::sync::Arc;
use uuid::Uuid;

use message_io::network::Endpoint;

use crate::RedisClient;
use crate::{ClientStatus, ConnId, NetPacketGuard, PacketType, TcpConn};
use crate::{PinkySwear, ServiceNetRs, ServiceRs};

use crate::service_net::create_redis_client;
use crate::service_net::tcp_conn_manager::{insert_connection, on_connection_established};

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
) -> Option<Arc<RedisClient>>
where
    T: ServiceRs + 'static,
{
    log::info!(
        "connect_to_redis: {} ... pass({}) dbindex({})",
        raddr,
        pass,
        dbindex
    );

    let (promise, pinky) = PinkySwear::<Option<Arc<RedisClient>>>::new();

    let conn_fn = |_1| {};
    let close_fn = |_1| {};

    // 投递到 srv_net 线程
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
            conn_fn,
            close_fn,
            &srv_net2,
        ));
        log::info!(
            "[connect_to_redis] [hd={}]({}) start connect to {} -- id<{}> ... ",
            cli.inner_hd(),
            cli.name(),
            raddr,
            cli.id(),
        );

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
                pinky.swear(Some(cli));
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
pub fn redis_client_make_new_conn(cli: &Arc<RedisClient>, hd: ConnId, endpoint: Endpoint) {
    //
    let packet_type = Atomic::new(PacketType::Redis);

    // insert tcp conn in srv net(同一线程便于观察 conn 生命周期)
    let cli2 = cli.clone();
    let func = move || {
        //
        let cli3 = cli2.clone();
        let connection_establish_fn = Box::new(move |conn: Arc<TcpConn>| {
            // 运行于 srv_net 线程
            assert!(conn.srv_net.is_in_service_thread());
            cli3.on_ll_connect(conn);
        });

        // use redis reply builder to handle input buffer
        let cli3 = cli2.clone();
        let connection_read_fn =
            Box::new(move |conn: Arc<TcpConn>, input_buffer: NetPacketGuard| {
                // 运行于 srv_net 线程
                assert!(conn.srv_net.is_in_service_thread());
                cli3.on_ll_input(conn, input_buffer);
            });

        //
        let cli3 = cli2.clone();
        let connection_lost_fn = Arc::new(move |hd: ConnId| {
            // 运行于 srv_net 线程
            assert!(cli3.srv_net().is_in_service_thread());
            cli3.on_ll_disconnect(hd);
        });

        //
        let netctrl = cli2.netctrl().clone();
        let srv_net = cli2.srv_net().clone();
        let srv = cli2.srv().clone();

        let conn = Arc::new(TcpConn {
            //
            hd,

            //
            endpoint,
            netctrl: netctrl.clone(),

            //
            packet_type,
            closed: Atomic::new(false),

            //
            srv: srv.clone(),
            srv_net: srv_net.clone(),

            //
            connection_establish_fn,
            connection_read_fn,
            connection_lost_fn: RwLock::new(connection_lost_fn),
        });

        // add conn
        insert_connection(srv_net.as_ref(), conn.hd, &conn.clone());

        // update inner hd for RedisClient
        {
            cli2.set_inner_hd(hd);
        }

        // redis_connection ok
        on_connection_established(conn);
    };
    cli.srv_net().run_in_service(Box::new(func));
}

///
pub fn redis_client_check_auto_reconnect(srv_net: &ServiceNetRs, hd: ConnId, cli_id: Uuid) {
    // 投递到 srv_net 线程
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
