#[cxx::bridge]
pub mod ffi_crypto {

    unsafe extern "C++" {
        include!("crypto_bindings.h");

        #[namespace = "commlib"]
        type BlowfishCfb64;

        #[namespace = "commlib"]
        fn new_blowfish() -> SharedPtr<BlowfishCfb64>;

        #[namespace = "commlib"]
        fn blowfish_set_key(bf: SharedPtr<BlowfishCfb64>, key: &[u8]);

        #[namespace = "commlib"]
        fn blowfish_set_init_vec(bf: SharedPtr<BlowfishCfb64>, init_vec: u64);

        #[namespace = "commlib"]
        fn blowfish_encrypt(bf: SharedPtr<BlowfishCfb64>, data: &[u8]) -> Vec<u8>;

        #[namespace = "commlib"]
        fn blowfish_decrypt(bf: SharedPtr<BlowfishCfb64>, data: &[u8]) -> Vec<u8>;
    }
}
