//!
//! Commlib: MD5
//!

use commlib_sys::ffi_hash::md5;

pub struct Md5();

impl Md5 {
    ///
    #[inline(always)]
    pub fn hash_slice(data: &[u8]) -> String {
        md5(data)
    }

    ///
    #[inline(always)]
    pub fn hash(data: &str) -> String {
        Self::hash_slice(data.as_bytes())
    }
}

/*
use openssl::hash::{Hasher, MessageDigest};

pub struct Md5();

impl Md5 {
    ///
    pub fn hash(data: &[u8]) -> String {
        let hasher_r = Hasher::new(MessageDigest::md5());
        match hasher_r {
            Ok(mut h) => {
                h.update(data).unwrap();
                let md5_r = h.finish();
                match md5_r {
                    Ok(res) => hex::encode(res),
                    Err(error) => {
                        log::error!("MD5 hash error: {:?}, data: {:?}", error, data);
                        "".to_owned()
                    }
                }
            }
            Err(error) => {
                log::error!("MD5 hasher error: {:?}, data: {:?}", error, data);
                "".to_owned()
            }
        }
    }
}
*/

#[cfg(test)]
mod tests {
    use hex::{self, FromHex};

    use commlib_sys::ffi_hash::{md5_block_size, md5_hash_bytes};

    use crate::utils::Md5;

    #[test]
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
            // treat string as hex bytes, such as "7F" means 0x7F
            let digest = Md5::hash_slice(Vec::from_hex(test.0).unwrap().as_slice());
            assert_eq!(digest, test.1);
        }

        assert_eq!(md5_block_size(), 64);
        assert_eq!(md5_hash_bytes(), 16);
    }
}
