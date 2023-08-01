use clap::{arg, Command};

fn main() {
    let arg_vec: Vec<std::ffi::OsString> = vec![
        "my_prog".into(),
        "--one".into(),
        "param1".into(),
        "--two".into(),
        "param2".into(),
    ];
    //let arg_vec: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let matches = Command::new("myprog")
        .version("1.0")
        .author("nneessh<nneessh@gmail.com>")
        .about("test")
        .arg(arg!(--two <VALUE>).required(true))
        .arg(arg!(--one <VALUE>).required(true))
        .get_matches_from(arg_vec);

    println!(
        "two: {:?}",
        matches.get_one::<String>("two").expect("required")
    );
    println!(
        "one: {:?}",
        matches.get_one::<String>("one").expect("required")
    );
}
