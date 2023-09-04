//!
//! CliManager
//!

use std::sync::Arc;

use commlib_sys::service_net::EncryptData;
use commlib_sys::{CmdId, ConnId, NetProxy, NodeState, PacketType, ServiceRs};
use commlib_sys::{G_SERVICE_NET, G_SERVICE_SIGNAL};

use crate::proto;
use prost::Message;

use crate::robot::{Robot, RobotManager, G_ROBOT_MANAGER};

use super::cli_conf::G_CLI_CONF;
use super::cli_service::CliService;
use super::cli_service::G_CLI_SERVICE;

thread_local! {
    ///
    pub static G_MAIN: std::cell::RefCell<CliManager> = {
        std::cell::RefCell::new(CliManager::new())
    };
}

///
pub struct CliManager {
    pub proxy: NetProxy,
}

impl CliManager {
    ///
    pub fn new() -> CliManager {
        CliManager {
            proxy: NetProxy::new(PacketType::Robot),
        }
    }

    ///
    pub fn init(&mut self, srv: &Arc<CliService>) -> bool {
        let handle = srv.get_handle();

        //
        self.proxy.set_packet_handler(
            proto::EnumMsgType::EncryptToken as CmdId,
            Self::handle_encrypt_token,
        );

        // ctrl-c stop, DEBUG ONLY
        G_SERVICE_SIGNAL.listen_sig_int(G_CLI_SERVICE.as_ref(), || {
            println!("WTF!!!!");
        });
        log::info!("\nGAME init ...\n");

        //
        app_helper::with_conf_mut!(G_CLI_CONF, cfg, { cfg.init(handle.xml_config()) });

        //
        handle.set_state(NodeState::Start);
        true
    }

    ///
    pub fn lazy_init(&mut self, srv: &Arc<CliService>) {
        log::info!("lazy init:");
    }

    /// 消息处理: encrypt token
    pub fn handle_encrypt_token(proxy: &mut NetProxy, hd: ConnId, cmd: CmdId, slice: &[u8]) {
        // 消息包加密 key
        let msg = proto::S2cEncryptToken::decode(slice).unwrap();

        let key = msg.token();
        proxy.set_encrypt_key(hd, key);

        G_ROBOT_MANAGER.with(|g| {
            let mut robot_mgr = g.borrow_mut();
            let rbt = robot_mgr.get_or_create_robot_by_hd(hd);
            rbt.borrow_mut().encrypt_key.extend_from_slice(key);

            // echo
            proxy.send_raw(G_SERVICE_NET.as_ref(), hd, cmd, slice);
            proxy.send_raw(G_SERVICE_NET.as_ref(), hd, cmd, slice);
        });
    }
}
