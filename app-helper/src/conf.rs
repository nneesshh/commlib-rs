//! Commlib: Conf
use commlib_sys::*;

pub struct Log {
    level: u32,                 // log级别
    path: std::path::PathBuf,   // 日志路径
    bipath: std::path::PathBuf, // BI日志路径
    console: bool,              // 是否输出到控制台
    async_queue: u32,           // 异步队列长度
}

pub struct WebUrl {
    api_addr: String,
    player_id_addr: String, // 用来获取新玩家 pid
    guild_id_addr: String,  // 用来获取新公会 id
    web_addr: String,       // web 服务地址
    web_addr_new: String,   // 新 web 服务地址

    update_player_addr: String, // 更新玩家基本信息
    update_guild_addr: String,  // 更新工会基本信息
    login_check_addr: String,   // 登录认证地址

    server_status_addr: String, // 区服状态信息上报
    server_config_url: String,  // 上报区服配置地址
    redeem_code_addr: String,   // 兑换码
    player_report_addr: String, // 上报玩家数据使用地址

    cross_relation_addr: String, // 跨服玩法关系配置

    wei_xin_pay_addr: String, // 微信小程序支付地址

    multi_log_addr: String,        // 多语言上报地址
    google_pay_check_addr: String, // google 订单校验地址
    firebase_notice: String,       // firebase 消息通知地址
}

pub struct Conf {
    job_params_: String, // 测试用例所需的工作参数字符串，用引号包围起来

    appname: String,
    etcfile: String,
    env_: String, // 当前的执行环境

    xmlreader: XmlReader,

    log: Log,
}

impl Conf {
    ///
    pub fn new() -> Conf {
        Conf {
            job_params_: "".to_owned(),

            appname: "server".to_owned(),
            etcfile: "".to_owned(),
            env_: "dev".to_owned(),

            xmlreader: XmlReader::new(),

            log: Log {
                level: spdlog::Level::Debug as u32,
                path: std::path::PathBuf::from("log"),
                bipath: std::path::PathBuf::from("bi"),
                console: true,
                async_queue: 8192_u32,
            },
        }
    }

    ///
    pub fn init() {
        let config_xml =
            xmlreader::XmlReader::read_file(std::path::Path::new("res/dragon.xml")).unwrap();
    }
}
