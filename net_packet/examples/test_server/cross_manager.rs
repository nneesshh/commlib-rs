use std::cell::UnsafeCell;
use std::sync::Arc;
use thread_local::ThreadLocal;

use commlib::{with_tls_mut, CmdId, NodeId, ServiceNetRs, ServiceRs, SpecialZone, ZoneId};

use app_helper::{Cluster, CrossStreamScheduler, RpcCall};

use crate::proto;
use prost::Message;

use crate::test_conf::G_TEST_CONF;

///
pub struct CrossManager {
    srv_net: Arc<ServiceNetRs>,
    srv: Arc<dyn ServiceRs>,

    tls_stream_scheduler: ThreadLocal<UnsafeCell<CrossStreamScheduler>>,

    tls_cross_rpc: ThreadLocal<UnsafeCell<RpcCall>>,
}

impl CrossManager {
    ///
    pub fn new(srv: &Arc<dyn ServiceRs>, srv_net: &Arc<ServiceNetRs>) -> Self {
        Self {
            srv: srv.clone(),
            srv_net: srv_net.clone(),

            tls_stream_scheduler: ThreadLocal::new(),

            tls_cross_rpc: ThreadLocal::new(),
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

    /// 通过 world 节点中转，上行发送到 cross
    pub fn send_to_cross_over_world<F, M>(
        self: &Arc<Self>,
        cluster: &mut Cluster,
        sp_zone: SpecialZone,
        node: NodeId,
        cmd: CmdId,
        msg: &M,
        cb: F,
    ) where
        F: Fn(CmdId, &[u8]) + Send + Sync + 'static,
        M: prost::Message,
    {
        //
        let mut source_node: NodeId = 0;
        with_tls_mut!(G_TEST_CONF, cfg, {
            source_node = cfg.my.id;
        });
        let msg_vec = msg.encode_to_vec();

        //
        let cross_rpc = self.get_cross_rpc();
        let rpcid = cross_rpc.add_cross_rpc_call_stub(cb);

        // 转换 SpecialZone 为 i32
        let zone = sp_zone as i8 as i32;

        //
        let mut req = proto::InnerCrossCall {
            node,
            zone,

            id: rpcid,
            r#type: cmd as i32,
            msg: msg_vec,

            source_node,
            source_zone: SpecialZone::Cross as i32,

            /// 回包标记(某个消息的回包还是发起包)
            resp: false,

            trans_zone: 0,
            trans_node: 0,
        };

        //
        cluster.send_to_world(req.encode_to_vec().as_slice());
    }

    /// 通过 world 节点中转，上行 return 回 cross
    pub fn return_to_cross_over_world(
        self: &Arc<Self>,
        cluster: &mut Cluster,
        sp_zone: SpecialZone,
        node: NodeId,
        channel: i32,
        rpcid: u64,
        cmd: CmdId,
        msg: String,
    ) {
        //
        let mut source_node: NodeId = 0;
        with_tls_mut!(G_TEST_CONF, cfg, {
            source_node = cfg.my.id;
        });
        let msg_vec = msg.encode_to_vec();

        // 转换 SpecialZone 为 i32
        let zone = sp_zone as i8 as i32;

        //
        let mut req = proto::InnerCrossCall {
            node,
            zone,

            id: rpcid,
            r#type: cmd as i32,
            msg: msg_vec,

            source_node,
            source_zone: SpecialZone::Cross as i32,

            /// 回包标记(某个消息的回包还是发起包)
            resp: false,

            trans_zone: 0,
            trans_node: 0,
        };

        //
        cluster.send_to_world(req.encode_to_vec().as_slice());
    }

    /// 通过 redis 中转，上行发送到 cross
    pub fn send_to_cross<F, M>(
        self: &Arc<Self>,
        sp_zone: SpecialZone,
        node: NodeId,
        channel: i32,
        cmd: CmdId,
        msg: &M,
        cb: F,
    ) where
        F: Fn(CmdId, &[u8]) + Send + Sync + 'static,
        M: prost::Message,
    {
        //
        let mut source_node: NodeId = 0;
        with_tls_mut!(G_TEST_CONF, cfg, {
            source_node = cfg.my.id;
        });
        let msg_vec = msg.encode_to_vec();

        //
        let cross_rpc = self.get_cross_rpc();
        let rpcid = cross_rpc.add_cross_rpc_call_stub(cb);

        // 转换 SpecialZone 为 i32
        let zone = sp_zone as i8 as i32;

        //
        let mut req = proto::InnerCrossCall {
            node,
            zone,

            id: rpcid,
            r#type: cmd as i32,
            msg: msg_vec,

            source_node,
            source_zone: SpecialZone::Cross as i32,

            /// 回包标记(某个消息的回包还是发起包)
            resp: false,

            trans_zone: 0,
            trans_node: 0,
        };

        //
        let scheduler = self.get_scheduler();
        scheduler.send_to_up_stream(sp_zone, node, channel, req.encode_to_vec())
    }

    /// 通过 redis 中转，上行 return 回 cross
    pub fn return_to_cross(
        self: &Arc<Self>,
        sp_zone: SpecialZone,
        node: NodeId,
        channel: i32,
        rpcid: u64,
        cmd: CmdId,
        msg: String,
    ) {
        //
        let mut source_node: NodeId = 0;
        with_tls_mut!(G_TEST_CONF, cfg, {
            source_node = cfg.my.id;
        });
        let msg_vec = msg.encode_to_vec();

        // 转换 SpecialZone 为 i32
        let zone = sp_zone as i8 as i32;

        //
        let mut req = proto::InnerCrossCall {
            node,
            zone,

            id: rpcid,
            r#type: cmd as i32,
            msg: msg_vec,

            source_node,
            source_zone: SpecialZone::Cross as i32,

            /// 回包标记(某个消息的回包还是发起包)
            resp: false,

            trans_zone: 0,
            trans_node: 0,
        };

        //
        let scheduler = self.get_scheduler();
        scheduler.send_to_up_stream(sp_zone, node, channel, req.encode_to_vec())
    }

    /// 通过 redis 中转，下行发送到小区
    pub fn send_to_zone<F, M>(
        self: &Arc<Self>,
        zone: ZoneId,
        node: NodeId,
        cmd: CmdId,
        msg: &M,
        cb: F,
    ) where
        F: Fn(CmdId, &[u8]) + Send + Sync + 'static,
        M: prost::Message,
    {
        //
        let mut source_node: NodeId = 0;
        with_tls_mut!(G_TEST_CONF, cfg, {
            source_node = cfg.my.id;
        });
        let msg_vec = msg.encode_to_vec();

        //
        let cross_rpc = self.get_cross_rpc();
        let rpcid = cross_rpc.add_cross_rpc_call_stub(cb);

        //
        let mut req = proto::InnerCrossCall {
            node,
            zone,

            id: rpcid,
            r#type: cmd as i32,
            msg: msg_vec,

            source_node,
            source_zone: SpecialZone::Cross as i32,

            /// 回包标记(某个消息的回包还是发起包)
            resp: false,

            trans_zone: 0,
            trans_node: 0,
        };

        //
        let scheduler = self.get_scheduler();
        scheduler.send_to_down_stream(zone, req.encode_to_vec())
    }

    /// 通过 redis 中转，下行 return 回小区
    pub fn return_to_zone(
        self: &Arc<Self>,
        zone: ZoneId,
        node: NodeId,
        rpcid: u64,
        cmd: CmdId,
        msg: String,
    ) {
        //
        let mut source_node: NodeId = 0;
        with_tls_mut!(G_TEST_CONF, cfg, {
            source_node = cfg.my.id;
        });
        let msg_vec = msg.encode_to_vec();

        //
        let mut req = proto::InnerCrossCall {
            node,
            zone,

            id: rpcid,
            r#type: cmd as i32,
            msg: msg_vec,

            source_node,
            source_zone: SpecialZone::Cross as i32,

            /// 回包标记(某个消息的回包还是发起包)
            resp: false,

            trans_zone: 0,
            trans_node: 0,
        };

        //
        let scheduler = self.get_scheduler();
        scheduler.send_to_down_stream(zone, req.encode_to_vec())
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

    #[inline(always)]
    fn get_cross_rpc<'a>(self: &'a Arc<Self>) -> &'a mut RpcCall {
        // 运行于 srv 线程
        assert!(self.srv.is_in_service_thread());

        let cross_rpc = self.tls_cross_rpc.get_or(|| {
            //
            UnsafeCell::new(RpcCall::new())
        });
        unsafe { &mut *(cross_rpc.get()) }
    }
}
