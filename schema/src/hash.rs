use std::fmt::Display;
use crate::{bytes_data, from_hex, Hash, HashFormatType, RgResult, SafeOption};
use crate::structs::{ErrorInfo, HashType};

use sha3::{Digest, Sha3_256};
use crate::proto_serde::ProtoSerde;


impl Hash {

    // Please don't use this, or be careful if using this as it's missing hash format.
    pub fn raw_bytes(&self) -> RgResult<Vec<u8>> {
        Ok(self.bytes.safe_get()?.clone().value)
    }
    pub fn vec(&self) -> Vec<u8> {
        self.proto_serialize()
    }
    pub fn hex(&self) -> String {
        hex::encode(self.vec())
    }

    pub fn new_from_proto(vec: Vec<u8>) -> RgResult<Self> {
        Hash::proto_deserialize(vec)
    }

    pub fn new_direct_transaction(vec: &Vec<u8>) -> Self {
        Self {
            bytes: bytes_data(vec.clone()),
            hash_format_type: HashFormatType::Sha3256 as i32,
            hash_type: HashType::Transaction as i32,
        }
    }

    pub fn validate_size(&self) -> Result<&Self, ErrorInfo> {
        let i = self.raw_bytes()?.len();
        // If switching to .vec() use 36
        if i == 32 {
            Ok(self)
        } else {
            Err(ErrorInfo::error_info("Invalid hash size"))
        }
    }


    // This function assumes a very basic type
    pub fn from_hex<S: Into<String>>(s: S) -> Result<Self, ErrorInfo> {
        let bytes = from_hex(s.into())?;
        let hash = Self::new_from_proto(bytes)?;
        hash.validate_size()?;
        Ok(hash)
    }

    pub fn from_string_calculate(s: &str) -> Self {
        Self::digest(s.as_bytes().to_vec())
    }

    pub fn digest(s: Vec<u8>) -> Self {
        Self::new_direct_transaction(&Sha3_256::digest(&s).to_vec())
    }

    pub fn div_mod(&self, bucket: usize) -> i64 {
        self.vec().iter().map(|i| i64::from(i.clone())).sum::<i64>() % (bucket as i64)
    }

    pub fn merkle_combine(&self, right: Hash) -> Self {
        let mut vec = self.vec();
        vec.extend(right.vec());
        Self::digest(vec)
    }

    pub fn checksum_no_calc(&self) -> Vec<u8> {
        self.vec()[0..4].to_vec()
    }

    pub fn checksum_hex(&self) -> String {
        hex::encode(self.checksum_no_calc())
    }

    pub fn xor_vec(&self, other: Hash) -> Vec<u8> {
        let v1 = self.vec();
        let v2 = other.vec();
        let xor_value: Vec<u8> = v1
            .iter()
            .zip(v2.iter())
            .map(|(&x1, &x2)| x1 ^ x2)
            .collect();
        xor_value
    }

    pub fn xor_distance(&self, other: Hash) -> u64 {
        let xor_value = self.xor_vec(other);
        let distance: u64 = xor_value.iter().map(|&byte| u64::from(byte)).sum();
        distance
    }

    pub fn new_checksum(s: &Vec<u8>) -> String {
        Self::digest(s.clone()).checksum_hex()
    }

}

#[test]
fn hash_rendering() {

    let h = Hash::from_string_calculate("test");
    println!("hash: {} {}", h.hex(), h.hex().len());
    let raw = hex::encode(h.raw_bytes().expect("works"));
    println!("hash raw bytes: {} len {}", raw, raw.len());
    // let mh = constants::HASHER.digest("test".as_bytes());
    // let mhb = hex::encode(mh.to_bytes());
    // let digestb = hex::encode(mh.digest());
    // let mh2 = Multihash::from_bytes(&*mh.to_bytes()).expect("multihash");
    // println!("mhb: {}", mhb);
    // println!("digest: {}", digestb);
    // println!("mh2: {}", hex::encode(mh2.to_bytes()));
    //

    // TODO: Parse versionInfo as a hash instead of a string.
    // let mut mhh = Multihash::default();
    // mhh.code() = Code::Sha2_256 as u64;
    // mhh.digest() = sha512("test".as_bytes()).to_vec();
}