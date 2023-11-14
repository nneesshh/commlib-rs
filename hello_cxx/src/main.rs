extern crate hello_cxx;

fn main() {
    extern "C" fn callback(sig: i32) {
        println!("Welcome back in Rust! Value={}", sig);
    }
    let cb1 = hello_cxx::SignalCallback(callback);
    let cb2 = hello_cxx::SignalCallback(callback);
    let cb3 = hello_cxx::SignalCallback(callback);

    unsafe {
        hello_cxx::ffi::init_signal_handlers(cb1, cb2, cb3);
    }

    for _ in 0.. {
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
