use std::cell::UnsafeCell;
use std::sync::Arc;

use crate::{ServiceNetRs, ServiceRs};

use super::NetPacketGuard;
use super::{ConnId, TcpConn};

thread_local! {
    static G_TCP_CONN_STORAGE: UnsafeCell<TcpConnStorage> = UnsafeCell::new(TcpConnStorage::new());
}

struct TcpConnStorage {
    /// tcp connection table
    conn_table: hashbrown::HashMap<ConnId, Arc<TcpConn>>,
}

impl TcpConnStorage {
    ///
    pub fn new() -> Self {
        Self {
            conn_table: hashbrown::HashMap::with_capacity(4096),
        }
    }
}

///
pub fn disconnect_connection<F>(
    srv_net: &Arc<ServiceNetRs>,
    hd: ConnId,
    close_cb: F,
    srv: &Arc<dyn ServiceRs>,
) where
    F: Fn(ConnId) + Send + Sync + 'static,
{
    // 在 srv_net 中运行
    let srv = srv.clone();
    let close_cb = Arc::new(close_cb);
    let func = move || {
        with_tls_mut!(G_TCP_CONN_STORAGE, g, {
            let mut is_conn_closed = false;
            if let Some(conn) = g.conn_table.get(&hd) {
                if conn.is_closed() {
                    is_conn_closed = true;
                } else {
                    // 修改 close_fn，运行 disconnect 回调函数: close_cb
                    let mut close_fn_mut = conn.close_fn.write();
                    (*close_fn_mut) = close_cb.clone();

                    // low level close
                    conn.close();
                }
            } else {
                log::error!(
                    "[hd={}] disconnect_connection failed!!! conn not found!!!",
                    hd,
                );
                is_conn_closed = true;
            }

            // 连接已经关闭，直接回调
            if is_conn_closed {
                srv.run_in_service(Box::new(move || {
                    (*close_cb)(hd);
                }));
            }
        });
    };
    srv_net.run_in_service(Box::new(func));
}

///
#[inline(always)]
pub fn handle_raw_data_event(
    srv_net: &Arc<ServiceNetRs>,
    hd: ConnId,
    input_buffer: NetPacketGuard,
) {
    // 在 srv_net 中运行
    let srv_net2 = srv_net.clone();
    let func = move || {
        on_got_raw_data(srv_net2.as_ref(), hd, input_buffer);
    };
    srv_net.run_in_service(Box::new(func));
}

///
pub fn handle_close_event(srv_net: &Arc<ServiceNetRs>, hd: ConnId) {
    // 在 srv_net 中运行
    let srv_net2 = srv_net.clone();
    let func = move || {
        on_connection_closed(srv_net2.as_ref(), hd);
    };
    srv_net.run_in_service(Box::new(func));
}

///
pub fn insert_connection(srv_net: &ServiceNetRs, hd: ConnId, conn: &Arc<TcpConn>) {
    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    with_tls_mut!(G_TCP_CONN_STORAGE, g, {
        log::info!("[hd={}] ++++++++ insert_connection", hd);
        g.conn_table.insert(hd, conn.clone());
    });
}

///
pub fn remove_connection(srv_net: &ServiceNetRs, hd: ConnId) -> Option<Arc<TcpConn>> {
    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    with_tls_mut!(G_TCP_CONN_STORAGE, g, {
        log::info!("[hd={}] -------- remove_connection", hd);
        g.conn_table.remove(&hd)
    })
}
///
#[inline(always)]
pub fn on_got_raw_data(srv_net: &ServiceNetRs, hd: ConnId, input_buffer: NetPacketGuard) {
    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    with_tls_mut!(G_TCP_CONN_STORAGE, g, {
        if let Some(conn) = g.conn_table.get(&hd) {
            // input buffer 数据读取处理
            (conn.read_fn)(conn, input_buffer)
        } else {
            //
            log::error!("[on_got_message][hd={}] conn not found!!!", hd);
        }
    });
}

///
pub fn on_connection_closed(srv_net: &ServiceNetRs, hd: ConnId) {
    // 运行于 srv_net 线程
    assert!(srv_net.is_in_service_thread());

    // remove conn always
    if let Some(conn) = remove_connection(srv_net, hd) {
        // trigger close_fn
        let f: Arc<dyn Fn(ConnId) + Send + Sync>;
        {
            let close_fn = conn.close_fn.read();
            f = (*close_fn).clone();
        }

        // 标记关闭
        conn.set_closed(true);

        //
        let srv = conn.srv.clone();
        srv.run_in_service(Box::new(move || {
            (f)(conn.hd);
        }));
    } else {
        //
        log::error!("[on_connection_closed][hd={}] conn not found!!!", hd);
    }
}

/// Trigger close_fn
pub fn run_close_fn(conn: &Arc<TcpConn>) {
    let conn = conn.clone();
    let f: Arc<dyn Fn(ConnId) + Send + Sync>;
    {
        let close_fn = conn.close_fn.read();
        f = (*close_fn).clone();
    }

    // 标记关闭
    conn.set_closed(true);

    //
    let srv = conn.srv.clone();
    srv.run_in_service(Box::new(move || {
        (f)(conn.hd);
    }));
}
