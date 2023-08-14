//! Commlib: CommlibDef

use crate::XmlReader;

/// 节点 id type
pub type NodeId = u64;

/// 区服 id type
pub type ZoneId = i32;

/// 平台 id type
pub type GroupId = u32;

/// Player id type
pub type PlayerId = u64;

/// 特殊区服 id
#[repr(C)]
pub enum SpecialZone {
    Cross = -1,            // 跨服操作：组队/实时 pvp
    WorldChatMng = -2,     // 跨服世界聊天管理节点
    WorldChatChannel = -3, // 跨服世界聊天频道节点
    Social = -4,           // 跨服社交节点
    Mechanics = -5,        // 跨服玩法节点
    Lobby = -6,            // 大厅管理节点
}

///
#[repr(C)]
pub struct NodeConf {
    pub id: NodeId,   // 节点 id
    pub addr: String, // 节点 ip
    pub port: u16,    // 节点端口
    pub index: i32,   // 节点分布索引
}

impl NodeConf {
    ///
    pub fn new() -> NodeConf {
        NodeConf {
            id: 0,
            addr: "".to_owned(),
            port: 0,
            index: 0,
        }
    }
}

/// 节点配置
pub const NODE_INDEX_MAX: usize = 16;
pub const NODE_ID_MIN: usize = 1000;

///
#[repr(C)]
pub struct RouteConf {
    pub gw: NodeConf,
    pub world: NodeConf,
    pub comm: NodeConf,
    pub db: NodeConf,

    pub scene_node_index: [NodeId; NODE_INDEX_MAX], // 场景节点
    pub scene_node_num: usize,                      // 场景节点数量

    pub lobby_node: NodeId, // 大厅节点 id
}

impl RouteConf {
    ///
    pub fn new() -> RouteConf {
        RouteConf {
            gw: NodeConf::new(),
            world: NodeConf::new(),
            comm: NodeConf::new(),
            db: NodeConf::new(),

            scene_node_index: [0; NODE_INDEX_MAX],
            scene_node_num: 0,

            lobby_node: 0,
        }
    }
}

///
#[repr(C)]
pub struct RedisConf {
    pub addr: String, // 节点 ip
    pub port: u16,    // 节点端口
    pub auth: String,
    pub db: usize,
}

impl RedisConf {
    ///
    pub fn new(addr: String, port: u16, auth: String, db: usize) -> RedisConf {
        RedisConf {
            addr,
            port,
            auth,
            db,
        }
    }
}

///
#[repr(C)]
pub struct DbAddr {
    pub addr: String, // 节点 ip
    pub port: u16,    // 节点端口

    pub user: String,
    pub pwd: String,
    pub db: String,
    pub charset: String,
}

impl DbAddr {
    ///
    pub fn new() -> DbAddr {
        DbAddr {
            addr: "".to_owned(),
            port: 0,

            user: "".to_owned(),
            pwd: "".to_owned(),
            db: "".to_owned(),
            charset: "utf8".to_owned(),
        }
    }

    ///
    pub fn from_xml(&mut self, xr: &XmlReader) {
        self.addr = xr.get_string(vec!["addr"], "");
        self.port = xr.get_u64(vec!["port"], 0) as u16;
        self.user = xr.get_string(vec!["user"], "");
        self.pwd = xr.get_string(vec!["pwd"], "");
        self.db = xr.get_string(vec!["db"], "");

        let charset = xr.get_string(vec!["charset"], "");
        if !charset.is_empty() {
            self.charset = charset;
        }
    }

    /// root:root@tcp(127.0.0.1:3306)/test?charset=utf8
    pub fn to_string(&self) -> String {
        format!(
            "{}:{}@tcp({}:{})/{}?charset={}",
            self.user, self.pwd, self.addr, self.port, self.db, self.charset
        )
    }
}
