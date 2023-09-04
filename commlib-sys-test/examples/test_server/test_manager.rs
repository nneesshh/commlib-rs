//!
//! TestManager
//!
use std::sync::Arc;

use commlib_sys::{CmdId, ConnId, NetProxy, NodeState, PacketType, ServiceRs};
use commlib_sys::{ENCRYPT_KEY_LEN, ENCRYPT_MAX_LEN};
use commlib_sys::{G_SERVICE_NET, G_SERVICE_SIGNAL};

use crate::proto;

use super::test_conf::G_TEST_CONF;
use super::test_service::TestService;
use super::test_service::G_TEST_SERVICE;

thread_local! {
    ///
    pub static G_MAIN: std::cell::RefCell<TestManager> = {
        std::cell::RefCell::new(TestManager::new())
    };
}

///
pub struct TestManager {
    pub c2s_proxy: NetProxy, // client to server
}

impl TestManager {
    ///
    pub fn new() -> TestManager {
        let mut c2s_proxy = NetProxy::new(PacketType::Client);
        c2s_proxy.set_encrypt_token_handler(|proxy, hd| {
            send_encrypt_token(proxy, hd);
        });

        TestManager {
            c2s_proxy: c2s_proxy,
        }
    }

    ///
    pub fn init(&mut self, srv: &Arc<TestService>) -> bool {
        let handle = srv.get_handle();

        // ctrl-c stop, DEBUG ONLY
        G_SERVICE_SIGNAL.listen_sig_int(G_TEST_SERVICE.as_ref(), || {
            println!("WTF!!!!");
        });
        log::info!("\nTest init ...\n");

        //
        app_helper::with_conf_mut!(G_TEST_CONF, cfg, { cfg.init(handle.xml_config()) });

        //
        handle.set_state(NodeState::Start);
        true
    }

    ///
    pub fn lazy_init(&mut self, srv: &Arc<TestService>) {
        log::info!("lazy init:");
    }
}

fn send_encrypt_token(proxy: &NetProxy, hd: ConnId) {
    let code_buff = vec![0_u8; ENCRYPT_KEY_LEN + ENCRYPT_MAX_LEN];

    let msg = proto::S2cEncryptToken {
        token: Some(code_buff.clone()),
    };

    // send encrypt key
    proxy.send_proto(
        G_SERVICE_NET.as_ref(),
        hd,
        proto::EnumMsgType::EncryptToken as CmdId,
        &msg,
    );
}
