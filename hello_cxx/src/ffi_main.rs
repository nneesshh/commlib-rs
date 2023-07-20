#[cxx::bridge]
pub mod ffi {

    extern "C++" {
        include!("signal.hpp");

        type SignalCallback = crate::SignalCallback;

        #[namespace = "commlib_cxx"]
        unsafe fn init_signal_handlers(
            cb1: SignalCallback,
            cb2: SignalCallback,
            cb3: SignalCallback,
        );

        #[namespace = "commlib_cxx"]
        unsafe fn new_abc();
    }
}
