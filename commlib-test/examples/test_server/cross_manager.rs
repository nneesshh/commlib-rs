use std::cell::UnsafeCell;
use std::sync::Arc;
use thread_local::ThreadLocal;

use commlib::{CmdId, ConnId, NodeId, RedisReply, ServiceNetRs, ServiceRs, ZoneId};

use app_helper::CrossStreamScheduler;

use crate::proto;
use prost::Message;

///
pub struct CrossManager {
    srv_net: Arc<ServiceNetRs>,
    srv: Arc<dyn ServiceRs>,

    tls_stream_scheduler: ThreadLocal<UnsafeCell<CrossStreamScheduler>>,
}

impl CrossManager {
    ///
    pub fn new(srv: &Arc<dyn ServiceRs>, srv_net: &Arc<ServiceNetRs>) -> Self {
        Self {
            srv: srv.clone(),
            srv_net: srv_net.clone(),

            tls_stream_scheduler: ThreadLocal::new(),
        }
    }

    ///
    pub fn on_cross_call(self: &Arc<Self>, time: u64, data: String) {
        //
        let msg = proto::InnerCrossCall::decode(data.as_bytes()).unwrap();
        log::info!("cross received cross call from zone:{} node:{} @{}=>zone:{} trans_zone:{} node:{}=>type:{}",
                msg.source_zone, msg.source_node, time, msg.zone, msg.trans_zone, msg.node, msg.r#type);

        // 处理
        if msg.trans_zone > 0 {
            // 跨服消息转发
            //self.trans_to_zone()
        }
    }

    ///
    pub fn init(self: &Arc<Self>) {
        let stream_scheduler = self.get_scheduler();
        stream_scheduler.init();
    }

    ///
    pub fn lazy_init(self: &Arc<Self>) {
        // Stream 接收线程延迟启动，防止消息处理回调函数尚未注册
        let cross_mgr = self.clone();
        let stream_message_fn = move |time: u64, data: String| {
            cross_mgr.on_cross_call(time, data);
        };
        let stream_scheduler = self.get_scheduler();
        stream_scheduler.lazy_init(stream_message_fn);
    }

    ///
    pub fn stop(self: &Arc<Self>) {
        let stream_scheduler = self.get_scheduler();
        stream_scheduler.stop();
    }

    /// 等待线程结束
    pub fn join_service(self: &Arc<Self>) {
        let stream_scheduler = self.get_scheduler();
        stream_scheduler.join_service();
    }

    ///
    pub fn trans_to_zone<F>(
        self: &Arc<Self>,
        zone: ZoneId,
        node: NodeId,
        cmd: CmdId,
        msg: String,
        rpc_return_cb: F,
    ) where
        F: Fn(ConnId, CmdId, &[u8]) + Send + Sync + 'static,
    {
        //
    }

    ///
    pub fn return_to_zone<F>(
        self: &Arc<Self>,
        zone: ZoneId,
        node: NodeId,
        cmd: CmdId,
        msg: String,
        rpc_return_cb: F,
    ) where
        F: Fn(ConnId, CmdId, &[u8]) + Send + Sync + 'static,
    {
        //
    }
    ////////////////////////////////////////////////////////////////

    #[inline(always)]
    fn get_scheduler<'a>(self: &'a Arc<Self>) -> &'a mut CrossStreamScheduler {
        // 运行于 srv 线程
        assert!(self.srv.is_in_service_thread());

        let scheduler = self.tls_stream_scheduler.get_or(|| {
            //
            UnsafeCell::new(CrossStreamScheduler::new(&self.srv, &self.srv_net))
        });
        unsafe { &mut *(scheduler.get()) }
    }
}
