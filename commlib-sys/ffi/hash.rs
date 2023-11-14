#[cxx::bridge]
pub mod ffi_hash {

    unsafe extern "C++" {
        include!("hash_bindings.h");

        #[namespace = "commlib"]
        fn md5(data: &[u8]) -> String;

        #[namespace = "commlib"]
        fn md5_block_size() -> usize;

        #[namespace = "commlib"]
        fn md5_hash_bytes() -> usize;
    }
}
