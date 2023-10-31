//! Commlib: MD5

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
