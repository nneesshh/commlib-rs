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

#[cfg(test)]
mod tests {
    use hex::{self, FromHex};

    use openssl::error::ErrorStack;
    use openssl::hash::{DigestBytes, Hasher, MessageDigest};
    use openssl::nid::Nid;
    use openssl::symm::{Cipher, Crypter, Mode};

    use crate::utils::Blowfish;
    use crate::utils::Md5;

    fn test_hash(hashtype: MessageDigest, hashtest: &(&str, &str)) {
        let res = hash(hashtype, &Vec::from_hex(hashtest.0).unwrap()).unwrap();
        assert_eq!(hex::encode(res), hashtest.1);
    }

    fn test_md5() {
        // Test vectors from http://www.nsrl.nist.gov/testdata/
        const MD5_TESTS: [(&str, &str); 13] = [
            ("", "d41d8cd98f00b204e9800998ecf8427e"),
            ("7F", "83acb6e67e50e31db6ed341dd2de1595"),
            ("EC9C", "0b07f0d4ca797d8ac58874f887cb0b68"),
            ("FEE57A", "e0d583171eb06d56198fc0ef22173907"),
            ("42F497E0", "7c430f178aefdf1487fee7144e9641e2"),
            ("C53B777F1C", "75ef141d64cb37ec423da2d9d440c925"),
            ("89D5B576327B", "ebbaf15eb0ed784c6faa9dc32831bf33"),
            ("5D4CCE781EB190", "ce175c4b08172019f05e6b5279889f2c"),
            ("81901FE94932D7B9", "cd4d2f62b8cdb3a0cf968a735a239281"),
            ("C9FFDEE7788EFB4EC9", "e0841a231ab698db30c6c0f3f246c014"),
            ("66AC4B7EBA95E53DC10B", "a3b3cea71910d9af56742aa0bb2fe329"),
            ("A510CD18F7A56852EB0319", "577e216843dd11573574d3fb209b97d8"),
            (
                "AAED18DBE8938C19ED734A8D",
                "6f80fb775f27e0a4ce5c2f42fc72c5f1",
            ),
        ];

        for test in MD5_TESTS.iter() {
            test_hash(MessageDigest::md5(), test);

            // treat string as hex bytes, such as "7F" means 0x7F
            let digest = Md5::hash(Vec::from_hex(test.0).unwrap().as_slice());
            assert_eq!(digest, test.1);
        }

        assert_eq!(MessageDigest::md5().block_size(), 64);
        assert_eq!(MessageDigest::md5().size(), 16);
        assert_eq!(MessageDigest::md5().type_().as_raw(), Nid::MD5.as_raw());
    }

    pub fn test_bf_cfb64() {
        let pt = "37363534333231204E6F77206973207468652074696D6520666F722000";
        let ct = "E73214A2822139CAF26ECF6D2EB9E76E3DA3DE04D1517200519D57A6C3";
        let key = "0123456789ABCDEFF0E1D2C3B4A59687";
        let iv = "FEDCBA9876543210";

        cipher_test_nopad(Cipher::bf_cfb64(), pt, ct, key, iv);

        let ciphertext = Blowfish::encrypt(key.as_bytes(), pt.as_bytes()).unwrap();
        let plaintext = Blowfish::decrypt(key.as_bytes(), &ciphertext.as_slice()).unwrap();
        assert_eq!(plaintext.as_slice(), pt.as_bytes());
    }

    fn hash(t: MessageDigest, data: &[u8]) -> Result<DigestBytes, ErrorStack> {
        let mut h = Hasher::new(t)?;
        h.update(data)?;
        h.finish()
    }

    fn cipher_test_nopad(ciphertype: Cipher, pt: &str, ct: &str, key: &str, iv: &str) {
        let pt = Vec::from_hex(pt).unwrap();
        let ct = Vec::from_hex(ct).unwrap();
        let key = Vec::from_hex(key).unwrap();
        let iv = Vec::from_hex(iv).unwrap();

        let computed = {
            let mut c = Crypter::new(ciphertype, Mode::Decrypt, &key, Some(&iv)).unwrap();
            c.pad(false);

            // Notice: remember to preserve 1 more byte for '\0'
            let mut out = vec![0; ct.len() + ciphertype.block_size() + 1];
            let count = c.update(&ct, &mut out).unwrap();
            let rest = c.finalize(&mut out[count..]).unwrap();
            out.truncate(count + rest);
            out
        };
        let expected = pt;

        if computed != expected {
            println!("Computed: {}", hex::encode(&computed));
            println!("Expected: {}", hex::encode(&expected));
            if computed.len() != expected.len() {
                println!(
                    "Lengths differ: {} in computed vs {} expected",
                    computed.len(),
                    expected.len()
                );
            }
            panic!("test failure");
        }
    }

    #[test]
    fn test_openssl() {
        // openssl is not thread safe for rust test
        crate::utils::ossl_init();

        test_md5();
        test_bf_cfb64();
    }
}
