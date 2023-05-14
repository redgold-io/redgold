use crate::{bytes_data, constants, Hash, HashFormatType, SafeBytesAccess};
use multihash::{Code, Multihash, MultihashDigest};
use crate::util::sha512;

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

    pub fn multihash(&self) -> Multihash {
        Multihash::from_bytes(&self.vec()).expect("multihash")
    }

}

#[test]
fn hash_rendering() {
    let mh = constants::HASHER.digest("test".as_bytes());
    let mhb = hex::encode(mh.to_bytes());
    let digestb = hex::encode(mh.digest());
    let mh2 = Multihash::from_bytes(&*mh.to_bytes()).expect("multihash");
    println!("mhb: {}", mhb);
    println!("digest: {}", digestb);
    println!("mh2: {}", hex::encode(mh2.to_bytes()));


    // TODO: Parse versionInfo as a hash instead of a string.
    // let mut mhh = Multihash::default();
    // mhh.code() = Code::Sha2_256 as u64;
    // mhh.digest() = sha512("test".as_bytes()).to_vec();
}