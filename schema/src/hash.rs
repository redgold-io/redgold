use crate::{bytes_data, constants, Hash, HashFormatType, SafeBytesAccess};
use multihash::{MultihashDigest};

impl Hash {
    pub fn vec(&self) -> Vec<u8> {
        self.safe_bytes().expect("a")
    }
    pub fn hex(&self) -> String {
        hex::encode(self.vec())
    }
    pub fn from_bytes_mh(vec: Vec<u8>) -> Self {
        Self {
            bytes: bytes_data(vec),
            hash_format_type: HashFormatType::Multihash as i32
        }
    }
    pub fn from_string(s: &str) -> Self {
        let mh = constants::HASHER.digest(s.as_bytes());
        mh.into()
    }
    pub fn calc_bytes(s: Vec<u8>) -> Self {
        let mh = constants::HASHER.digest(&s);
        mh.into()
    }

    pub fn merkle_combine(&self, right: Hash) -> Self {
        let mut vec = self.vec();
        vec.extend(right.vec());
        Self::calc_bytes(vec)
    }

    pub fn ecdsa_short_signing_bytes(&self) -> Vec<u8> {
        self.vec()[0..32].to_vec()
    }

}