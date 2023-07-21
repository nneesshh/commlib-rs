
extern crate commlib_sys;

fn main() {

  let srv_net = commlib_sys::ffi_net::service_net_new(0);
  //srv_net.OnConnection(srv)

  let srv_net2 = 

    commlib_sys::ffi_sig::new_abc();
    
    extern "C" fn callback(sig: i32) {
        println!("Welcome back in Rust! Value={}", sig);
    }
    let cb1 = commlib_sys::SignalCallback(callback);
    let cb2 = commlib_sys::SignalCallback(callback);
    let cb3 = commlib_sys::SignalCallback(callback);

    commlib_sys::ffi_sig::init_signal_handlers(cb1, cb2, cb3);

  for _ in 0.. {
    std::thread::sleep(std::time::Duration::from_millis(100));
  }
}

