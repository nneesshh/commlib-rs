
extern crate commlib_sys;

fn main() {
    unsafe {
        commlib_sys::ffi::new_abc();
    }
   
    extern "C" fn callback(sig: i32) {
        println!("Welcome back in Rust! Value={}", sig);
    }
    let cb1 = commlib_sys::SignalCallback(callback);
    let cb2 = commlib_sys::SignalCallback(callback);
    let cb3 = commlib_sys::SignalCallback(callback);

    unsafe {
        commlib_sys::ffi::init_signal_handlers(cb1, cb2, cb3);
    }
  
  for _ in 0.. {
    std::thread::sleep(std::time::Duration::from_millis(100));
  }
}

