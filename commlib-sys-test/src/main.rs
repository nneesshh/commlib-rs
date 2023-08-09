use app_helper::App;
mod test_service;

fn main() {

    let x: fn();
    
    let x = 1234567_i64;
    let y = x.as_ptr();

    let CRLF:&[u8;2] = b"\r\n";
    use bytes::{BytesMut, BufMut};

    let mut buf = BytesMut::with_capacity(1024);
    buf.put(&b"hello world"[..]);
    buf.put_u16(1234);
    

    let a = buf.split();
    assert_eq!(a, b"hello world\x04\xD2"[..]);
    
    buf.put(&b"goodbye world"[..]);
    
    let b = buf.split();
    assert_eq!(b, b"goodbye world"[..]);
    
    assert_eq!(buf.capacity(), 998);

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

    let arg_vec: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let mut app = App::new(&arg_vec, "test");
    app.attach(|| test_service::G_TEST_SERVICE.as_ref());
    app.run();
}
