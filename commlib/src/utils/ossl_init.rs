//! Commlib: ossl_init

std::thread_local! {
    pub static G_OSSL_PROVIDER_LEGACY: openssl::provider::Provider = {
        let provider_r = openssl::provider::Provider::try_load(None, "legacy", true);
        match provider_r {
            Ok(prov) => prov,
            Err(error) => {
                log::error!("openssl legacy provider error: {:?}", error);
                std::unreachable!()
            }
        }
    }
}

///
#[inline(always)]
pub fn ossl_init() {
    ossl_init_legacy();
}

#[inline(always)]
fn ossl_init_legacy() {
    G_OSSL_PROVIDER_LEGACY.with(|_prov| {
        // do something
        log::info!("ossl_init_legacy ok.");
    });
}
