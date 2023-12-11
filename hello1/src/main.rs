use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use log::Level;
use net_packet::take_packet;
use parking_lot::Mutex;

use message_io::network::*;
use message_io::node::{self, NodeHandler};
use message_io::node_event::NodeEvent;
use message_io::util::thread::NamespacedThread;
use rand::{SeedableRng, Rng};

const LOCAL_ADDR: &'static str = "127.0.0.1:12345";
const MIN_MESSAGE: &'static [u8] = &[42];
const BIG_MESSAGE_SIZE: usize = 1024 * 1024 * 8; // 8MB

// Common error messages
const TIMEOUT_EVENT_RECV_ERR: &'static str = "Timeout, but an event was expected.";

lazy_static::lazy_static! {
    static ref TIMEOUT: Duration = Duration::from_millis(15000);
    static ref LOCALHOST_CONN_TIMEOUT: Duration = Duration::from_millis(10000);
}

fn start_echo_server(
    transport: Transport,
    expected_clients: usize,
) -> (NamespacedThread<()>, SocketAddr) {
    let (tx, rx) = crossbeam_channel::bounded(1);
    let mut thread = NamespacedThread::spawn("test-server", move || {
        let messages_received = AtomicUsize::new(0);
        let disconnections = AtomicUsize::new(0);
        let clients = Mutex::new(HashSet::new());

        let (mux, handler) = node::split();

        handler.listen(
            transport,
            LOCAL_ADDR,
            Box::new(move |_h: &NodeHandler, ret| {
                //
                if let Ok((_listenr_id, server_addr)) = ret {
                    tx.send(server_addr).unwrap();
                }
            }),
        );

        let listener_id_opt = Mutex::new(None);
        let mut task = node::node_listener_for_each_async(mux, &handler, move |h, e| {
            match e {
                NodeEvent::Waker(_) => {
                    //
                    log::trace!("waker");
                }
                NodeEvent::Network(net_event) => match net_event {
                    NetEvent::Connected(..) => unreachable!(),
                    NetEvent::Accepted(endpoint, id) => {
                        {
                            let mut guard = listener_id_opt.lock();
                            *guard = Some(id);
                        }
                        match transport.is_connection_oriented() {
                            true => {
                                //
                                let mut guard = clients.lock();
                                assert!(guard.insert(endpoint));
                            }
                            false => unreachable!(),
                        }
                    }
                    NetEvent::Message(endpoint, data) => {
                        assert_eq!(MIN_MESSAGE, data.peek());

                        h.send(endpoint, data);

                        messages_received.fetch_add(1, Ordering::Relaxed);

                        if !transport.is_connection_oriented() {
                            // We assume here that if the protocol is not
                            // connection-oriented it will no create a resource.
                            // The remote will be managed from the listener resource
                            {
                                let guard = listener_id_opt.lock();
                                let listener_id = guard.unwrap();
                                assert_eq!(listener_id, endpoint.resource_id());
                            }
                            if messages_received.load(Ordering::Relaxed) == expected_clients {
                                h.stop() //Exit from thread.
                            }
                        }
                    }
                    NetEvent::Disconnected(endpoint) => {
                        match transport.is_connection_oriented() {
                            true => {
                                disconnections.fetch_add(1, Ordering::Relaxed);

                                //
                                {
                                    let mut guard = clients.lock();
                                    assert!(guard.remove(&endpoint));
                                }
                                if disconnections.load(Ordering::Relaxed) == expected_clients {
                                    assert_eq!(
                                        expected_clients,
                                        messages_received.load(Ordering::Relaxed)
                                    );
                                    {
                                        let guard = clients.lock();
                                        let clients_len = guard.len();
                                        assert_eq!(0, clients_len);
                                    }
                                    h.stop() //Exit from thread.
                                }
                            }
                            false => unreachable!(),
                        }
                    }
                },
            }
        });
        task.wait();
    });

    thread.join();
    let server_addr = rx.recv_timeout(*TIMEOUT).expect(TIMEOUT_EVENT_RECV_ERR);
    (thread, server_addr)
}

fn create_remove_listener_with_connection() {
    let (mux, handler) = node::split();

    handler.listen(Transport::Tcp, "127.0.0.1:0", move |h, ret| {
        if let Ok((_listener_id, addr)) = ret {
            h.connect(Transport::Tcp, addr, |_h, _| {});
        }
    });

    let was_accepted = Arc::new(Mutex::new(false));
    let was_accepted2 = was_accepted.clone();
    let mut task = node::node_listener_for_each_async(mux, &handler, move |h, e| {
        let net_event = e.network();
        match net_event {
            NetEvent::Connected(..) => {
                //
                println!("Connected");
            }
            NetEvent::Accepted(_, listener_id) => {
                h.close(listener_id);
                h.close(listener_id);
                let mut guard = was_accepted2.lock();
                *guard = true;
                h.stop();
            }
            _ => {}
        }
    });
    task.wait();

    let guard = was_accepted.lock();
    assert!(*guard);
}

fn main() {
    let log_path = std::path::PathBuf::from("log");
    let log_level = my_logger::LogLevel::Info as u16;
    my_logger::init(&log_path, "hello1", log_level, false);

    //start_echo_server(Transport::Tcp, 100);

    create_remove_listener_with_connection();
}
