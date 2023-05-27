use crate::structs::{Address, AddressInfo, AddressType, Error, ErrorInfo, Hash, UtxoEntry};
use crate::{bytes_data, error_info, from_hex, SafeBytesAccess};
use crate::{error_message, structs};
use bitcoin::secp256k1::{PublicKey};
use bitcoin::util::base58;
use std::io::Write;
use sha3::Sha3_224;

use sha3::Digest;

// impl fromstr for address etc. impl tostring
impl Into<Address> for structs::PublicKey {
    fn into(self) -> Address {
        Address::from_struct_public(&self).expect("some")
    }
}

impl Into<Address> for Vec<u8> {
    fn into(self) -> Address {
        Address::address_data(self).expect("some")
    }
}


impl Address {
    pub fn parse<S: Into<String>>(addr: S) -> Result<Address, ErrorInfo> {
        let s = addr.into();

        Self::from_hex(s)
        // // TODO: Address validation function here honestly
        // if s.len() < 5 {
        //     return Err(error_message(
        //         Error::AddressDecodeFailure,
        //         format!("Address minimum string length failure on address: {}", s.clone()),
        //     ));
        // }
        // // this slice 3 is unsafe.
        // let address_vec = base58::from_check(&s.clone()[3..]).map_err(|e| {
        //     error_message(
        //         Error::AddressDecodeFailure,
        //         format!("Base58 checked address decoding failure on {} {}", s.clone(), e.to_string()),
        //     )
        // })?;
        // Ok(address_vec.into())
    }
    pub fn render_string(&self) -> Result<String, ErrorInfo> {
        let result = self.address.safe_bytes()?;
        Ok(Self::address_to_str(&result))
    }
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Address, ErrorInfo> {
        let addr = Self::new(bytes);
        addr.verify_checksum()?;

        Ok(addr)
    }

    pub fn from_public(pk: &PublicKey) -> Result<Address, ErrorInfo> {
        Self::from_bytes(Self::address_function(pk))
    }

    pub fn from_struct_public(pk: &structs::PublicKey) -> Result<Address, ErrorInfo> {
        Self::from_bytes(Self::hash(&pk.bytes.safe_bytes()?))
    }

    pub fn with_checksum(bytes: Vec<u8>) -> Vec<u8> {
        let checksum_bytes = Hash::calc_bytes(bytes.clone()).vec();
        let mut res: Vec<u8> = Vec::new();
        res.extend_from_slice(&bytes);
        res.extend_from_slice(&checksum_bytes[0..4]);
        res
    }

    pub fn hash(buf: &[u8]) -> Vec<u8> {
        let bytes = Sha3_224::digest(buf).to_vec();
        Self::with_checksum(bytes)
    }

    pub fn verify_length(&self) -> Result<(), ErrorInfo> {
        let i = self.address.safe_bytes()?.len();
        if i != 32 {
            Err(error_info(format!("Invalid address length: {:?}", i)))?;
        }
        Ok(())
    }

    pub fn verify_checksum(&self) -> Result<(), ErrorInfo> {
        self.verify_length()?;
        let bytes = self.address.safe_bytes()?;
        if Self::with_checksum(bytes[0..28].to_vec()) != bytes {
            Err(error_info("Invalid address checksum bytes"))?;
        }
        Ok(())
    }

    pub fn address_function(pk: &PublicKey) -> Vec<u8> {
        return Self::hash(pk.serialize().as_ref());
    }

    pub fn multi_address_function(pk: &Vec<PublicKey>) -> Vec<u8> {
        let mut v: Vec<u8> = Vec::new();
        for pki in pk {
            v.extend(pki.serialize().as_ref())
        }
        return Self::hash(&*v);
    }

    pub fn str_to_address(s: String) -> Vec<u8> {
        hex::decode(s).expect("hex")
        // return base58::from_check(&s[3..]).unwrap();
    }

    pub fn address_to_str(a: &Vec<u8>) -> String {
        // let mut b = base58::check_encode_slice(&*a);
        // b.insert_str(0, "rg1");
        // return b;
        hex::encode(a)
    }

    pub fn address(pk: &PublicKey) -> Vec<u8> {
        return Self::address_function(pk);
    }

    pub fn multi_address(public_key: &Vec<PublicKey>) -> Vec<u8> {
        return Self::multi_address_function(public_key);
    }

    pub fn address_data(address: Vec<u8>) -> Option<Address> {
        Some(Self::new(address))
    }

    pub fn new(address: Vec<u8>) -> Address {
        Address {
            address: bytes_data(address),
            address_type: AddressType::Sha3224ChecksumPublic as i32,
        }
    }


    fn from_hex(p0: String) -> Result<Address, ErrorInfo> {
        let bytes = from_hex(p0)?;
        Address::from_bytes(bytes)
    }
}


//
// #[test]
// fn address_hash_test() {
//     let tc = TestConstants::new();
//     let a = address_function(&tc.public);
//     let b = base58::check_encode_slice(&*a);
//     let c = base58::from_check(&*b).unwrap();
//     let mut bb = b.clone();
//     bb.insert_str(0, "rg1");
//     let cc = address_to_str(&a);
//     // println!("{:?}", a);
//     // println!("{:?}", b);
//     // println!("{:?}", bb);
//     // println!("{:?}", bb.len());
//     // println!("{:?}", cc);
//     assert_eq!(a, c);
//     assert_eq!(a, str_to_address(cc.clone()));
//     assert_eq!("rg1M7NTPxADbn4iV1wWPaRg3LYL4ZCLQLfR5", cc);
// }


impl AddressInfo {
    pub fn from_utxo_entries(address: Address, entries: Vec<UtxoEntry>) -> Self {
        let mut bal: i64 = 0;
        for r in &entries {
            if let Some(o) = &r.output {
                if let Some(d) = &o.data {
                    if let Some(a) = d.amount {
                        bal += a;
                    }
                }
            }
        }
        AddressInfo {
            address: Some(address.clone()),
            utxo_entries: entries,
            balance: bal
        }
    }
}