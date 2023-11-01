use atomic::{Atomic, Ordering};
use parking_lot::RwLock;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

use commlib::utils::string_to_value;
use commlib::{connect_to_redis, redis, NodeId, SpecialZone};
use commlib::{RedisClient, RedisReply, RedisReplyType, ServiceNetRs, ServiceRs, ZoneId};

use crate::cross_stream_keys::{
    get_down_stream_name, get_up_stream_name, make_streams_ids_pair, stream_id_for_zone,
};
use crate::G_CONF;

const READ_MSG_COUNT: usize = 10;
const WAIT_DELAY_SECONDS: u64 = 5;
const STREAM_PENDING_ID: &str = "0-0";

///
pub struct CrossStreamScheduler {
    srv_net: Arc<ServiceNetRs>,
    srv: Arc<dyn ServiceRs>,

    stop: Arc<Atomic<bool>>,

    cli_send_to_queue_opt: Option<Arc<RedisClient>>,

    cli_receive_from_queue_join_handle_opt: RwLock<Option<JoinHandle<()>>>,
}

impl CrossStreamScheduler {
    ///
    pub fn new(srv: &Arc<dyn ServiceRs>, srv_net: &Arc<ServiceNetRs>) -> Self {
        Self {
            srv: srv.clone(),
            srv_net: srv_net.clone(),

            stop: Arc::new(Atomic::new(false)),

            cli_send_to_queue_opt: None,

            cli_receive_from_queue_join_handle_opt: RwLock::new(None),
        }
    }

    ///
    pub fn init(&mut self) {
        let g_conf = G_CONF.load();
        let raddr = std::format!("{}:{}", g_conf.queue_redis.addr, g_conf.queue_redis.port);
        let pass = g_conf.queue_redis.pass.as_str();
        let dbindex = g_conf.queue_redis.dbindex;

        //
        log::info!("init send_to_queue client ...");
        self.cli_send_to_queue_opt =
            connect_to_redis(&self.srv, raddr.as_str(), pass, dbindex, &self.srv_net);
        if self.cli_send_to_queue_opt.is_none() {
            log::info!("init send_to_queue client failed!!!");

            // commlib exit
            std::panic!("cross stream scheduler error: init send_to_queue client failed!!!");
        }
    }

    ///
    pub fn lazy_init<F>(&mut self, stream_message_fn: F)
    where
        F: Fn(u64, String) + Send + Sync + 'static,
    {
        // Stream 接收线程延迟启动，防止消息处理回调函数尚未注册
        let srv_net = self.srv_net.clone();
        let srv = self.srv.clone();
        let stop = self.stop.clone();
        let join_handle = std::thread::spawn(move || {
            //
            run_receive_and_parse(&srv, stream_message_fn, stop, &srv_net);
        });

        //
        {
            let mut join_handle_opt_mut = self.cli_receive_from_queue_join_handle_opt.write();
            (*join_handle_opt_mut) = Some(join_handle);
        }
    }

    ///
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    /// 等待线程结束
    pub fn join_service(&self) {
        let mut join_handle_opt_mut = self.cli_receive_from_queue_join_handle_opt.write();
        if let Some(join_handle) = join_handle_opt_mut.take() {
            join_handle.join().unwrap();
        }
    }

    /// 通过redis中，向上发送到跨服
    pub fn send_to_up_stream(
        &self,
        sp_zone: SpecialZone,
        node: NodeId,
        channel: i32,
        data: Vec<u8>,
    ) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let key = get_up_stream_name(sp_zone, node, channel);
        let data = unsafe { String::from_utf8_unchecked(data) };

        //
        redis::xadd(
            self.cli_send_to_queue_opt.as_ref().unwrap(),
            key.as_str(),
            "*",
            vec!["time".to_owned(), now.to_string(), "msg".to_owned(), data],
            move |rpl| {
                //
                if rpl.is_error() {
                    log::error!(
                        "cross(XADD) send_to_up_stream sp_zone {} node {} channel {} error: {}",
                        sp_zone as i8,
                        node,
                        channel,
                        rpl.error()
                    );
                } else {
                    //
                    log::debug!(
                        "cross(XADD) send_to_stream sp_zone {} node {} channel {} ok -- reply:{:?}",
                        sp_zone as i8,
                        node,
                        channel,
                        rpl
                    );
                }
            },
        )
    }

    /// 通过redis中转到下行区服
    pub fn send_to_down_stream(&self, zone: ZoneId, data: Vec<u8>) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let key = get_down_stream_name(zone);
        let data = unsafe { String::from_utf8_unchecked(data) };

        //
        redis::xadd(
            self.cli_send_to_queue_opt.as_ref().unwrap(),
            key.as_str(),
            "*",
            vec!["time".to_owned(), now.to_string(), "msg".to_owned(), data],
            move |rpl| {
                //
                if rpl.is_error() {
                    log::error!(
                        "cross(XADD) send_to_down_stream zone {} error: {}",
                        zone,
                        rpl.error()
                    );
                } else {
                    //
                    log::debug!(
                        "cross(XADD) send_to_down_stream zone {} ok -- reply:{:?}",
                        zone,
                        rpl
                    );
                }
            },
        )
    }
}

fn on_receive_stream_mesage_vec(
    srv: &Arc<dyn ServiceRs>,
    stream_message_fn: &Arc<dyn Fn(u64, String) + Send + Sync>,
    stream_message_vec: &Vec<RedisReply>,
) {
    //
    for stream_pair in stream_message_vec {
        let stream_pair_vec = stream_pair.as_array();
        assert!(stream_pair_vec.len() >= 2);

        let _stream_name = stream_pair_vec[0].as_string();
        let msgs = &stream_pair_vec[1];

        if msgs.is_array() {
            for msg in msgs.as_array() {
                let msg_vec = msg.as_array();
                assert!(msg_vec.len() >= 2);

                let _id = msg_vec[0].as_string();
                let fields = &msg_vec[1];
                if fields.is_array() {
                    let fields_vec = fields.as_array();

                    //
                    let mut data: String = "".to_owned();
                    let mut time: u64 = 0;

                    let i = 0_usize;
                    while i + 1 < fields_vec.len() {
                        let field_name = fields_vec[i].as_string();
                        if field_name == "time" {
                            time = string_to_value(fields_vec[i + 1].as_string());
                        } else {
                            data = fields_vec[i + 1].as_string().to_owned();
                        }
                    }

                    // stream msg 扔回到 srv 线程去处理
                    let stream_message_fn2 = stream_message_fn.clone();
                    srv.run_in_service(Box::new(move || {
                        //
                        (*stream_message_fn2)(time, data);
                    }));
                }
            }
        }
    }
}

fn run_receive_and_parse<F>(
    srv: &Arc<dyn ServiceRs>,
    stream_message_fn: F,
    stop: Arc<Atomic<bool>>,
    srv_net: &Arc<ServiceNetRs>,
) where
    F: Fn(u64, String) + Send + Sync + 'static,
{
    //
    let g_conf = G_CONF.load();
    let raddr = std::format!("{}:{}", g_conf.queue_redis.addr, g_conf.queue_redis.port);
    let pass = g_conf.queue_redis.pass.as_str();
    let dbindex = g_conf.queue_redis.dbindex;

    let zone = g_conf.zone_id;

    //
    log::info!("init receive_from_queue client ...");
    let cli_receive_from_queue_opt = connect_to_redis(srv, raddr.as_str(), pass, dbindex, &srv_net);
    match cli_receive_from_queue_opt {
        Some(cli) => {
            //
            do_receive_and_parse(srv, stream_message_fn, stop, zone, &cli);
        }
        None => {
            log::info!("init receive_from_queue client failed!!!");

            // commlib exit
            std::panic!("cross stream scheduler error: init receive_from_queue client failed!!!");
        }
    }
}

fn do_receive_and_parse<F>(
    srv: &Arc<dyn ServiceRs>,
    stream_message_fn: F,
    stop: Arc<Atomic<bool>>,
    zone: ZoneId,
    cli: &Arc<RedisClient>,
) where
    F: Fn(u64, String) + Send + Sync + 'static,
{
    //
    let stream_message_fn: Arc<dyn Fn(u64, String) + Send + Sync> = Arc::new(stream_message_fn);

    // 所有的消息队列
    let mut stream_ids = hashbrown::HashMap::<String, String>::new();
    let stream = stream_id_for_zone(zone);
    stream_ids.insert(stream, STREAM_PENDING_ID.to_owned());

    let count = READ_MSG_COUNT;
    let block = WAIT_DELAY_SECONDS * 1000; // ms

    let streams_ids_pair = make_streams_ids_pair(&stream_ids);

    while !stop.load(Ordering::Relaxed) {
        if !cli.is_connected() {
            log::info!(
                "redis queue(XREAD) not ready, wait and retry after {} seconds ...",
                WAIT_DELAY_SECONDS
            );
            std::thread::sleep(std::time::Duration::from_secs(WAIT_DELAY_SECONDS));
            continue;
        }

        // read (必须 blocking 调用 XREAD 才能在当前线程接收到 reply，若使用异步调用 reply 将自动投递到 srv 线程，而非当前线程)
        let rpl = redis::xread_blocking(&cli, count, block, &streams_ids_pair);

        // process
        match rpl.reply_type() {
            RedisReplyType::Error => {
                // retry after N second
                const DELAY_SECS: u64 = 5_u64;
                log::error!(
                    "read redis queue(XREAD) error: {}, retry after {} seconds ...",
                    rpl.error(),
                    DELAY_SECS
                );
                std::thread::sleep(std::time::Duration::from_secs(DELAY_SECS));
            }

            RedisReplyType::Null => {
                // timeout
                log::debug!("read redis queue(XREAD) waiting block timeout, continue waiting ...")
            }

            RedisReplyType::Array => {
                // 每个 stream message 的结果
                let stream_message_vec = rpl.as_array();
                on_receive_stream_mesage_vec(srv, &stream_message_fn, stream_message_vec);
            }
            _ => {
                //
                std::unreachable!()
            }
        }
    }
}
