use clap::{arg, Command};

fn main() {
    // panic hook
    std::panic::set_hook(Box::new(|panic_info| {
        println!(
            "panic info: {:?}, {:?}, panic occurred in {:?}",
            panic_info.payload().downcast_ref::<&str>(),
            panic_info.to_string(),
            panic_info.location()
        );
        log::error!(
            "panic info: {:?}, {:?}, panic occurred in {:?}",
            panic_info.payload().downcast_ref::<&str>(),
            panic_info.to_string(),
            panic_info.location()
        );
    }));

    // let arg_vec: Vec<std::ffi::OsString> = vec![
    //     "my_prog".into(),
    //     "-o".into(),
    //     "param1".into(),
    //     "-t".into(),
    //     "param2".into(),
    // ];
    let arg_vec: Vec<String> = std::env::args().collect();
    let matches = Command::new("myprog")
        .version("1.0")
        .author("nneessh<nneessh@gmail.com>")
        .about("test")
        .arg(arg!(-t --two <VALUE>).required(true))
        .arg(arg!(-o --one <VALUE>).required(true))
        .get_matches_from(arg_vec);

    println!(
        "two: {:?}",
        matches.get_one::<String>("two").expect("required")
    );
    println!(
        "one: {:?}",
        matches.get_one::<String>("one").expect("required")
    );

    // let arg_vec: Vec<std::ffi::OsString> = std::env::args_os().collect();
    // let matches = clap::Command::new("myprog")
    //         .version("1.0")
    //         .author("nneessh<nneessh@gmail.com>")
    //         .about("app-helper::conf")
    //         .arg(clap::arg!(-c --config <FILE> "配置文件地址").required(false).default_value(""))
    //         .arg(clap::arg!(-n --nodeid <VALUE> "启动节点").required(false).default_value("0"))
    //         .arg(clap::arg!(-l --loglevel <VALUE> "日志等级").required(false).default_value("0"))
    //         .arg(clap::arg!(-a --api <VALUE> "node api 地址").required(false).default_value(""))
    //         .arg(clap::arg!(-s --servername <STRING> "服务器名称").required(false).default_value(""))
    //         .arg(clap::arg!(-z --zone <VALUE> "区服id").required(false).default_value("0"))
    //         .arg(clap::arg!(-g --group <VALUE> "服务器组（平台）").required(false).default_value("0"))
    //         .arg(clap::arg!(-v --version <VALUE> "版本号").required(false).default_value(""))
    //         .arg(clap::arg!(-j --"job-params" <VALUE> "测试用例所需的工作参数字符串，用引号包围起来").required(false).default_value(""))
    //         .get_matches_from(arg_vec);
}
