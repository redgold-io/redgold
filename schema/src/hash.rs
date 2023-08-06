use std::fmt::{Display, Formatter};
use crate::{bytes_data, constants, from_hex, Hash, HashFormatType, RgResult, SafeBytesAccess};
use crate::structs::{ErrorInfo, HashType};

use sha3::{Digest, Sha3_256};



/// Please note this is the direct constructor and does not perform an actual hash
impl Into<Hash> for Vec<u8> {
    fn into(self) -> Hash {
        Hash::new(self)
    }
}


impl Hash {
    pub fn vec(&self) -> Vec<u8> {
        self.safe_bytes().expect("a")
    }
    pub fn hex(&self) -> String {
        hex::encode(self.vec())
    }
    // TODO: From other types as well
    pub fn new(vec: Vec<u8>) -> Self {
        Self {
            bytes: bytes_data(vec),
            hash_format_type: HashFormatType::Sha3256 as i32,
            hash_type: HashType::Transaction as i32,
        }
    }

    pub fn validate_size(&self) -> Result<&Self, ErrorInfo> {
        if self.safe_bytes()?.len() == 32 {
            Ok(self)
        } else {
            Err(ErrorInfo::error_info("Invalid hash size"))
        }
    }

    pub fn from_hex<S: Into<String>>(s: S) -> Result<Self, ErrorInfo> {
        // TODO: Validate size
        let hash = Self::new(from_hex(s.into())?);
        hash.validate_size()?;
        Ok(hash)
    }

    pub fn from_string_calculate(s: &str) -> Self {
        Self::digest(s.as_bytes().to_vec())
    }

    pub fn digest(s: Vec<u8>) -> Self {
        Self::new(Sha3_256::digest(&s).to_vec())
    }

    pub fn merkle_combine(&self, right: Hash) -> Self {
        let mut vec = self.vec();
        vec.extend(right.vec());
        Self::digest(vec)
    }

    pub fn checksum(&self) -> Result<Vec<u8>, ErrorInfo> {
        Ok(self.safe_bytes()?[0..4].to_vec())
    }

    pub fn checksum_hex(&self) -> Result<String, ErrorInfo> {
        Ok(hex::encode(self.checksum()?))
    }

    pub fn xor_vec(&self, other: Hash) -> RgResult<Vec<u8>> {
        let v1 = self.safe_bytes()?;
        let v2 = other.safe_bytes()?;
        let xor_value: Vec<u8> = v1
            .iter()
            .zip(v2.iter())
            .map(|(&x1, &x2)| x1 ^ x2)
            .collect();
        Ok(xor_value)
    }

    pub fn xor_distance(&self, other: Hash) -> RgResult<u64> {
        let xor_value = self.xor_vec(other)?;
        let distance: u64 = xor_value.iter().map(|&byte| u64::from(byte)).sum();
        Ok(distance)
    }

}

#[test]
fn hash_rendering() {

    let h = Hash::from_string_calculate("test");
    println!("hash: {}", h.hex());
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