use std::collections::HashSet;
use std::net::SocketAddr;
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

fn start_echo_client_manager(
    transport: Transport,
    server_addr: SocketAddr,
    clients_number: usize,
) {
    let mut thread = NamespacedThread::spawn("test-client", move || {
        let (mux, handler) = node::split();

        let clients = Mutex::new(HashSet::new());
        let received = AtomicUsize::new(0);

        for _ in 0..clients_number {
            handler.connect(transport, server_addr, Box::new(|_h:&NodeHandler, _| {}));
        }

        let mut task = node::node_listener_for_each_async(
            mux,
            &handler,
            Box::new(move |h:&NodeHandler, e| {
                //
                match e {
                    NodeEvent::Waker(_) => panic!("{}", TIMEOUT_EVENT_RECV_ERR),
                    NodeEvent::Network(net_event) => match net_event {
                        NetEvent::Connected(server, status) => {
                            assert!(status);
                            let mut buffer = take_packet(MIN_MESSAGE.len());
                            buffer.append_slice(MIN_MESSAGE);
                            h.send(server, buffer);

                            //
                            {
                                let mut guard = clients.lock();
                                assert!(guard.insert(server));
                            }
                        }
                        NetEvent::Message(endpoint, data) => {
                            //
                            {
                                let mut guard = clients.lock();
                                assert!(guard.remove(&endpoint));
                            }

                            assert_eq!(MIN_MESSAGE, data.peek());
                            h.close(endpoint.resource_id());

                            received.fetch_add(1, Ordering::Relaxed);
                            if received.load(Ordering::Relaxed) == clients_number {
                                h.stop(); //Exit from thread.
                            }
                        }
                        NetEvent::Accepted(..) => unreachable!(),
                        NetEvent::Disconnected(_) => unreachable!(),
                    },
                }
            }),
        );
        task.wait();
    });

    thread.join();
}

fn main() {
    let log_path = std::path::PathBuf::from("log");
    let log_level = my_logger::LogLevel::Info as u16;
    my_logger::init(&log_path, "hello2", log_level, false);

    let server_addr = LOCAL_ADDR.parse().unwrap();
    start_echo_client_manager(Transport::Tcp, server_addr, 100);
}
