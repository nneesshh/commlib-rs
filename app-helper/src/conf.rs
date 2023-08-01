//! Commlib: Conf
use commlib_sys::*;
use roxmltree::Node;

use crate::conf;

pub type NodeId = u64;
pub type ZoneId = i32;
pub type GroupId = u32;

pub const TEST_NODE: NodeId = 999;

#[allow(dead_code)]
pub struct Log {
    level: u32,                 // log级别
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
pub struct Conf {
    pub job_params_: String, // 测试用例所需的工作参数字符串，用引号包围起来

    pub appname: String,
    pub etcfile: std::ffi::OsString, // 配置文件名称
    pub env_: String,                // 当前的执行环境

    pub log: Log,
    pub url: WebUrl,

    pub workdir: std::path::PathBuf, // 启动目录
    pub command: std::path::PathBuf, // 启动命令

    pub node_id: NodeId,   // 区服节点 id
    pub zone_id: ZoneId,   // 区服 id
    pub group_id: GroupId, // 平台 id

    pub version: String,     // 服务器版本号
    pub version_check: bool, // 是否检查版本号

    pub local_xml_nodes: hashbrown::HashMap<NodeId, XmlReader>, // xml 配置数据
}

impl Conf {
    ///
    pub fn new() -> Conf {
        Conf {
            job_params_: "".to_owned(),

            appname: "server".to_owned(),
            etcfile: std::ffi::OsString::default(),
            env_: "dev".to_owned(),

            log: Log {
                level: spdlog::Level::Debug as u32,
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

            version: "".to_owned(),
            version_check: false,

            local_xml_nodes: hashbrown::HashMap::new(),
        }
    }

    ///
    pub fn init(&mut self, arg_vec: &Vec<std::ffi::OsString>, srv_name: &str) {
        // 读取一下当前的执行环境
        let env_r = get_run_env();
        match env_r {
            Ok(content) => self.env_ = content,
            Err(err) => {}
        }

        // 解析命令行参数
        let matches = clap::Command::new("myprog")
            .version("1.0")
            .author("nneessh<nneessh@gmail.com>")
            .about("app-helper::conf")
            .arg(clap::arg!(-c --config <FILE> "配置文件地址").required(false).default_value(""))
            .arg(clap::arg!(-n --nodeid <VALUE> "启动节点").required(false).default_value("0"))
            .arg(clap::arg!(-l --loglevel <VALUE> "日志等级").required(false).default_value("0"))
            .arg(clap::arg!(-a --api <VALUE> "node api 地址").required(false).default_value(""))
            .arg(clap::arg!(-s --servername <STRING> "服务器名称").required(false).default_value(""))
            .arg(clap::arg!(-z --zone <VALUE> "区服id").required(false).default_value("0"))
            .arg(clap::arg!(-g --group <VALUE> "服务器组（平台）").required(false).default_value("0"))
            .arg(clap::arg!(-v --version <VALUE> "版本号").required(false).default_value(""))
            .arg(clap::arg!(-j --"job-params" <VALUE> "测试用例所需的工作参数字符串，用引号包围起来").required(false).default_value(""))
            .get_matches_from(arg_vec);

        // 启动目录
        self.workdir = std::env::current_dir().unwrap();

        // 启动命令
        self.command = std::env::current_exe().unwrap();

        // 配置文件位置，先从参数获取，再从默认位置
        self.etcfile = matches
            .get_one::<std::ffi::OsString>("c")
            .unwrap()
            .to_owned();
        if (self.etcfile.is_empty()) {
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
        self.job_params_ = matches.get_one::<String>("j").unwrap().to_owned();

        //
        let loglevel = matches.get_one::<u32>("l").unwrap();
        if *loglevel > 0 {
            self.log.level = *loglevel;
        }

        //
        self.url.api_addr = matches.get_one::<String>("a").unwrap().to_owned();

        //
        self.node_id = *matches.get_one::<NodeId>("n").unwrap();
        self.zone_id = *matches.get_one::<ZoneId>("z").unwrap();
        self.group_id = *matches.get_one::<GroupId>("g").unwrap();

        // 设置 appname (等价于设置 log file name)
        let server_name = matches.get_one::<String>("s").unwrap();
        if !server_name.is_empty() {
            self.appname = server_name.to_owned();
        } else {
            self.appname = srv_name.to_owned();
        }

        //
        let ver = matches.get_one::<String>("v").unwrap();
        if !ver.is_empty() {
            self.version = ver.to_owned();
        }

        // 从 etcfile(xml 格式) 中读取配置信息
        if !self.etcfile.is_empty() {
            let config_xml =
                xmlreader::XmlReader::read_file(std::path::Path::new("res/dragon.xml")).unwrap();
            Self::read_config_from_xml( &config_xml, srv_name);
        }

        // node_id must match xml_node
        if self.node_id > 0 {
            if TEST_NODE == self.node_id {
                // test node do nothing
            } else {
                let found = false;
                for (k, v) in &self.local_xml_nodes {

                }
            }
        }
    }

    fn read_config_from_xml(config_xml:& xmlreader::XmlReader, srv_name: &str) {

    }
}

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
