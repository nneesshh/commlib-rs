//! Commlib: Conf

use commlib::utils::{split_string_to_set, string_to_value, Base64};
use commlib::{GroupId, NodeId, XmlReader, ZoneId};

pub const TEST_NODE: NodeId = 999;

/// 获取当前执行环境，正式环境目录结构
/// dragon-game/
///     env.dat
///            /bin/
///                 执行文件
pub fn get_run_env() -> Result<String, String> {
    const ENV_FILE_PATH: &str = "../env.dat";
    let path = std::path::Path::new(ENV_FILE_PATH);
    let content_r = std::fs::read_to_string(path);
    match content_r {
        Ok(content) => Ok(content),
        Err(e) => {
            let errmsg = format!("read env file({:?}) error: {}.", path, e);
            println!("{errmsg}");
            Err(errmsg)
        }
    }
}

#[allow(dead_code)]
pub struct Log {
    level: u16,                 // log级别
    path: std::path::PathBuf,   // 日志路径
    bipath: std::path::PathBuf, // BI日志路径
    console: bool,              // 是否输出到控制台
    async_queue: u32,           // 异步队列长度
}

#[allow(dead_code)]
pub struct WebUrl {
    pub api_addr: String,
    pub player_id_addr: String, // 用来获取新玩家 pid
    pub guild_id_addr: String,  // 用来获取新公会 id
    pub web_addr: String,       // web 服务地址
    pub web_addr_new: String,   // 新 web 服务地址

    pub update_player_addr: String, // 更新玩家基本信息
    pub update_guild_addr: String,  // 更新工会基本信息
    pub login_check_addr: String,   // 登录认证地址

    pub server_status_addr: String, // 区服状态信息上报
    pub server_config_url: String,  // 上报区服配置地址
    pub redeem_code_addr: String,   // 兑换码
    pub player_report_addr: String, // 上报玩家数据使用地址

    pub cross_relation_addr: String, // 跨服玩法关系配置

    pub wei_xin_pay_addr: String, // 微信小程序支付地址

    pub multi_log_addr: String,        // 多语言上报地址
    pub google_pay_check_addr: String, // google 订单校验地址
    pub firebase_notice: String,       // firebase 消息通知地址

    pub auth_token: std::ffi::OsString, // http auth token
}

#[allow(dead_code)]
pub struct RedisAddr {
    pub addr: String,
    pub port: u16,
    pub pass: String,
    pub dbindex: isize,
}

impl RedisAddr {
    ///
    pub fn from_xml(&mut self, xr: &XmlReader) {
        self.addr = xr.get(vec!["addr"], "127.0.0.1".to_owned());
        self.port = xr.get_u64(vec!["port"], 6379_u64) as u16;
        self.pass = xr.get(vec!["auth"], "".to_owned());
        self.dbindex = xr.get_u64(vec!["db"], 0_u64) as isize;
    }
}

#[allow(dead_code)]
pub struct Conf {
    pub job_params_: String, // 测试用例所需的工作参数字符串，用引号包围起来

    pub appname: String,
    pub etcfile: std::ffi::OsString, // 配置文件名称
    pub env_: String,                // 当前的执行环境
    pub encrypt_token: Vec<u8>,      // 协议包加密密钥

    pub http_port: u16, // http 服务端口号

    pub log: Log,
    pub url: WebUrl,

    pub workdir: std::path::PathBuf, // 启动目录
    pub command: std::path::PathBuf, // 启动命令

    pub node_id: NodeId,   // 区服节点 id
    pub zone_id: ZoneId,   // 区服 id
    pub group_id: GroupId, // 平台 id

    pub limit_players: u32, // 玩家注册数限制

    pub version: String,     // 服务器版本号
    pub version_check: bool, // 是否检查版本号

    pub db_redis: RedisAddr,    // db cache 用
    pub queue_redis: RedisAddr, // 消息队列用

    pub local_xml_nodes: hashbrown::HashMap<NodeId, XmlReader>, // xml 配置数据

    pub cross_zones: hashbrown::HashSet<ZoneId>, // 同一跨服内的区服列表
}

impl Conf {
    ///
    pub fn new() -> Conf {
        Conf {
            job_params_: "".to_owned(),

            appname: "server".to_owned(),
            etcfile: std::ffi::OsString::default(),
            env_: "dev".to_owned(),
            encrypt_token: Vec::new(),

            http_port: 8081,

            log: Log {
                level: my_logger::LogLevel::Debug as u16,
                path: std::path::PathBuf::from("log"),
                bipath: std::path::PathBuf::from("bi"),
                console: true,
                async_queue: 8192_u32,
            },

            url: WebUrl {
                api_addr: "".to_owned(),
                player_id_addr: "".to_owned(),
                guild_id_addr: "".to_owned(),
                web_addr: "".to_owned(),
                web_addr_new: "".to_owned(),
                update_player_addr: "".to_owned(),
                update_guild_addr: "".to_owned(),
                login_check_addr: "".to_owned(),
                server_status_addr: "".to_owned(),
                server_config_url: "".to_owned(),
                redeem_code_addr: "".to_owned(),
                player_report_addr: "".to_owned(),
                cross_relation_addr: "".to_owned(),
                wei_xin_pay_addr: "".to_owned(),
                multi_log_addr: "".to_owned(),
                google_pay_check_addr: "".to_owned(),
                firebase_notice: "".to_owned(),

                auth_token: std::ffi::OsString::default(),
            },

            workdir: std::path::PathBuf::default(),
            command: std::path::PathBuf::default(),

            node_id: 0,
            zone_id: 0,
            group_id: 0,

            limit_players: 20000,

            version: "".to_owned(),
            version_check: false,

            db_redis: RedisAddr {
                addr: "".to_owned(),
                port: 0,
                pass: "".to_owned(),
                dbindex: 0,
            },
            queue_redis: RedisAddr {
                addr: "".to_owned(),
                port: 0,
                pass: "".to_owned(),
                dbindex: 0,
            },

            local_xml_nodes: hashbrown::HashMap::new(),

            cross_zones: hashbrown::HashSet::new(),
        }
    }

    ///
    pub fn init(&mut self, arg_vec: &Vec<std::ffi::OsString>, srv_name: &str) {
        // 读取一下当前的执行环境
        let env_r = get_run_env();
        match env_r {
            Ok(content) => self.env_ = content,
            Err(_err) => {}
        }

        // 解析命令行参数
        let matches = clap::Command::new("myprog")
            .author("nneessh<nneessh@gmail.com>")
            .about("app-helper::conf")
            .arg(clap::arg!(-c --config <FILE> "配置文件地址").value_parser(clap::value_parser!(String)).required(false).default_value(""))
            .arg(clap::arg!(-n --nodeid <VALUE> "启动节点").value_parser(clap::value_parser!(NodeId)).required(false).default_value("0"))
            .arg(clap::arg!(-l --loglevel <VALUE> "日志等级").value_parser(clap::value_parser!(u32)).required(false).default_value("0"))
            .arg(clap::arg!(-a --api <VALUE> "node api 地址").value_parser(clap::value_parser!(String)).required(false).default_value(""))
            .arg(clap::arg!(-s --servername <STRING> "服务器名称").value_parser(clap::value_parser!(String)).required(false).default_value(""))
            .arg(clap::arg!(-z --zone <VALUE> "区服id").value_parser(clap::value_parser!(ZoneId)).required(false).default_value("0"))
            .arg(clap::arg!(-g --group <VALUE> "服务器组（平台）").value_parser(clap::value_parser!(GroupId)).required(false).default_value("0"))
            .arg(clap::arg!(-v --version <VALUE> "版本号").value_parser(clap::value_parser!(String)).required(false).default_value(""))
            .arg(clap::arg!(-j --"job-params" <VALUE> "测试用例所需的工作参数字符串，用引号包围起来").value_parser(clap::value_parser!(String)).required(false).default_value(""))
            .get_matches_from(arg_vec);

        // 启动目录
        self.workdir = std::env::current_dir().unwrap();

        // 启动命令
        self.command = std::env::current_exe().unwrap();

        // 配置文件位置，先从参数获取，再从默认位置
        let etcfile = matches.get_one::<String>("config").unwrap().to_owned();
        if !etcfile.is_empty() {
            self.etcfile = std::ffi::OsString::from(etcfile.trim());
        } else {
            const ETCFILE_DEFAULT: &str = "res/dragon.xml";
            const DRAGON_XML_CFG_ENV: &str = "DRAGON_XML_CFG";
            if let Some(cfg_env) = std::env::var_os(DRAGON_XML_CFG_ENV) {
                self.etcfile = cfg_env;
            } else {
                self.etcfile = std::ffi::OsString::from(ETCFILE_DEFAULT);
            }
        }

        // http token: 优先使用环境变量
        const DRAGON_HTTP_TOKEN_ENV: &str = "DRAGON_HTTP_TOKEN_ENV";
        if let Some(auth_token) = std::env::var_os(DRAGON_HTTP_TOKEN_ENV) {
            self.url.auth_token = auth_token;
        }

        //
        self.job_params_ = matches.get_one::<String>("job-params").unwrap().to_owned();

        //
        let loglevel = matches.get_one::<u32>("loglevel").unwrap();
        if *loglevel > 0 {
            self.log.level = (*loglevel) as u16;
        }

        //
        self.url.api_addr = matches.get_one::<String>("api").unwrap().to_owned();

        //
        self.node_id = *matches.get_one::<NodeId>("nodeid").unwrap();
        self.zone_id = *matches.get_one::<ZoneId>("zone").unwrap();
        self.group_id = *matches.get_one::<GroupId>("group").unwrap();

        // 设置 appname (等价于设置 log file name)
        let server_name = matches.get_one::<String>("servername").unwrap();
        if !server_name.is_empty() {
            self.appname = server_name.to_owned();
        } else {
            self.appname = srv_name.to_owned();
        }

        //
        let ver = matches.get_one::<String>("version").unwrap();
        if !ver.is_empty() {
            self.version = ver.to_owned();
        }

        // 从 etcfile(xml 格式) 中读取配置信息
        if !self.etcfile.is_empty() {
            let config_xml = XmlReader::read_file(std::path::Path::new(&self.etcfile)).unwrap();
            self.read_config_from_xml(&config_xml, srv_name);
        }

        // node_id must match xml_node
        if self.node_id > 0 {
            if srv_name.is_empty() && TEST_NODE == self.node_id {
                // test node do nothing
            } else {
                //
                let found = {
                    if let Some(_) = self.local_xml_nodes.get(&self.node_id) {
                        true
                    } else {
                        false
                    }
                };

                // can't find xml node
                if !found {
                    // commlib exit
                    std::panic!("node xml error");
                    //std::process::exit(0);
                }
            }
        } else {
            // commlib exit
            std::panic!("null node");
            //std::process::exit(0);
        }

        // 保证 includes 包含自己
        self.cross_zones.insert(self.zone_id);
    }

    ///
    pub fn is_valid_zone(&self, zone: ZoneId) -> bool {
        if zone == self.zone_id {
            true
        } else {
            for z in &self.cross_zones {
                if zone == *z {
                    return true;
                }
            }
            false
        }
    }

    ///
    pub fn get_xml_node(&self, node_id: NodeId) -> Option<&XmlReader> {
        self.local_xml_nodes.get(&node_id)
    }

    ///
    fn read_config_from_xml(&mut self, config_xml: &XmlReader, srv_name: &str) {
        // use command line first
        if self.zone_id == 0 {
            self.zone_id = config_xml.get::<ZoneId>(vec!["zone"], 0);
        }

        // use command line first
        if self.group_id == 0 {
            self.group_id = config_xml.get::<GroupId>(vec!["group"], 0);
        }

        // use command line first
        if self.version.is_empty() {
            self.version = config_xml.get::<String>(vec!["version"], "".to_owned());
        }

        //
        self.version_check = config_xml.get::<bool>(vec!["version", "check"], true);

        // use command line first
        if self.cross_zones.len() == 0 {
            let zones = config_xml.get::<String>(vec!["zones"], "".to_owned());
            self.cross_zones = split_string_to_set::<ZoneId>(&zones, ",");
        }

        //
        self.limit_players = config_xml.get::<u32>(vec!["limit_players"], self.limit_players);

        //
        let token_str = config_xml.get::<String>(vec!["encrypt_token"], "".to_owned());
        if !token_str.is_empty() {
            self.encrypt_token = Base64::decode(token_str).unwrap();
        }

        //
        let http_port_str = config_xml.get::<String>(vec!["http_port"], "".to_owned());
        if !http_port_str.is_empty() {
            self.http_port = string_to_value(http_port_str.as_str());
        }

        //
        self.db_redis
            .from_xml(config_xml.get_child(vec!["redis", "db"]).unwrap());
        self.queue_redis
            .from_xml(config_xml.get_child(vec!["redis", "queue"]).unwrap());

        //
        if srv_name.is_empty() {
            if TEST_NODE != self.node_id {
                // TEST NODE
                std::panic!("TEST NODE ID must be {}", TEST_NODE);
                //std::process::exit(0);
            }
        } else {
            // node id must match xml_node
            if let Some(xml_nodes) = config_xml.get_children(vec!["node"]) {
                for xml_node in xml_nodes {
                    let nid = xml_node.get_u64(vec!["id"], 0);
                    let name = xml_node.get_string(vec!["name"], "");

                    self.local_xml_nodes.insert(nid, xml_node.to_owned());

                    if self.node_id == 0 && name.eq_ignore_ascii_case(srv_name) {
                        self.node_id = nid;
                    }
                }
            }
        }
    }
}
