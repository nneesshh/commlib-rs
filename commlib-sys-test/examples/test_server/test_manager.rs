//!
//! TestManager
//!

use commlib_sys::G_SERVICE_NET;
use commlib_sys::{CmdId, ConnId, NetProxy, PacketType};
use commlib_sys::{ENCRYPT_KEY_LEN, ENCRYPT_MAX_LEN};

use crate::proto;

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
        let mut c2s_proxy = NetProxy::new(PacketType::Client, &G_SERVICE_NET);
        c2s_proxy.set_encrypt_token_handler(|proxy, hd| {
            send_encrypt_token(proxy, hd);
        });

        TestManager {
            c2s_proxy: c2s_proxy,
        }
    }
}

fn send_encrypt_token(proxy: &NetProxy, hd: ConnId) {
    let code_buff = vec![0_u8; ENCRYPT_KEY_LEN + ENCRYPT_MAX_LEN];

    let msg = proto::S2cEncryptToken {
        token: Some(code_buff.clone()),
    };

    // send encrypt key
    proxy.send_proto(hd, proto::EnumMsgType::EncryptToken as CmdId, &msg);
}
