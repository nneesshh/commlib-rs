use atomic::{Atomic, Ordering};

use net_packet::CmdId;

///
pub type CrossRpcReturnCallback = Box<dyn FnOnce(CmdId, &[u8]) + Send + Sync>;
pub struct RpcCallStub {
    pub rpc_return_cb: CrossRpcReturnCallback,
}

///
pub struct RpcCall {
    pub next_rpc_uuid: Atomic<u64>,
    pub waiting_table: hashbrown::HashMap<u64, RpcCallStub>,
}

impl RpcCall {
    ///
    pub fn new() -> Self {
        Self {
            next_rpc_uuid: Atomic::new(1),
            waiting_table: hashbrown::HashMap::new(),
        }
    }

    ///
    pub fn add_cross_rpc_call_stub<F>(&mut self, cb: F) -> u64
    where
        F: Fn(CmdId, &[u8]) + Send + Sync + 'static,
    {
        //
        let rpc_uuid = self.next_rpc_uuid.fetch_add(1, Ordering::Relaxed);

        //
        self.waiting_table.insert(
            rpc_uuid,
            RpcCallStub {
                rpc_return_cb: Box::new(cb),
            },
        );
        rpc_uuid
    }
}
