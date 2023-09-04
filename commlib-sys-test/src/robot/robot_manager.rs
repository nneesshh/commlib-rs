//!
//! RobotManager
//!

use atomic::{Atomic, Ordering};
use prost::Message;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use commlib_sys::{CmdId, ConnId, NetProxy, NodeState, PacketType, ServiceRs};
use commlib_sys::{G_SERVICE_NET, G_SERVICE_SIGNAL};

use crate::proto;

use crate::cli_conf::G_CLI_CONF;
use crate::cli_service::CliService;
use crate::cli_service::G_CLI_SERVICE;

thread_local! {
    ///
    pub static G_ROBOT_MANAGER: std::cell::RefCell<RobotManager> = {
        std::cell::RefCell::new(RobotManager::new())
    };
}

///
pub type RobotId = usize;

///
pub struct Robot {
    pub id: usize,
    pub encrypt_key: Vec<u8>,
}

///
pub struct RobotManager {
    pub robot_table: hashbrown::HashMap<RobotId, Rc<RefCell<Robot>>>,
    pub hd_2_rid_table: hashbrown::HashMap<ConnId, RobotId>,
    pub next_rid: Atomic<RobotId>,
}

impl RobotManager {
    ///
    pub fn new() -> RobotManager {
        Self {
            robot_table: hashbrown::HashMap::new(),
            hd_2_rid_table: hashbrown::HashMap::new(),
            next_rid: Atomic::new(0),
        }
    }

    ///
    pub fn get_robot(&self, rid: &RobotId) -> Option<Rc<RefCell<Robot>>> {
        if let Some(rbt) = self.robot_table.get(rid) {
            Some(rbt.clone())
        } else {
            None
        }
    }

    ///
    pub fn get_or_create_robot_by_hd(&mut self, hd: ConnId) -> Rc<RefCell<Robot>> {
        let mut robot_opt: Option<Rc<RefCell<Robot>>> = None;
        if let Some(rid) = self.hd_2_rid_table.get(&hd) {
            if let Some(robot) = self.get_robot(rid) {
                robot_opt = Some(robot.clone());
            }
        }

        match robot_opt {
            Some(robot) => robot,
            None => {
                // new robot
                let rid = self.next_rid.load(Ordering::Relaxed);
                let robot = Rc::new(RefCell::new(Robot {
                    id: rid,
                    encrypt_key: Vec::new(),
                }));

                //
                self.robot_table.insert(rid, robot.clone());
                self.hd_2_rid_table.insert(hd, rid);

                //
                robot
            }
        }
    }
}
