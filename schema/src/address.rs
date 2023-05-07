use crate::structs::{Address, AddressInfo, AddressType, Error, ErrorInfo, UtxoEntry};
use crate::{bytes_data, SafeBytesAccess};
use crate::{error_message, structs};
use bitcoin::secp256k1::{PublicKey};
use bitcoin::util::base58;
use std::io::Write;


// impl fromstr for address etc. impl tostring
impl Into<Address> for structs::PublicKey {
    fn into(self) -> Address {
        Address {
            address: self.bytes.map(|b| {
                let mut b2 = b.clone();
                b2.value = address_function_buf(&b.value);
                b2
            }),
            address_type: Some(AddressType::StandardKeyhash as i32),
        }
    }
}

impl Address {
    pub fn parse<S: Into<String>>(addr: S) -> Result<Address, ErrorInfo> {
        let s = addr.into();
        // TODO: Address validation function here honestly
        if s.len() < 5 {
            return Err(error_message(
                Error::AddressDecodeFailure,
                format!("Address minimum string length failure on address: {}", s.clone()),
            ));
        }
        // this slice 3 is unsafe.
        let address_vec = base58::from_check(&s.clone()[3..]).map_err(|e| {
            error_message(
                Error::AddressDecodeFailure,
                format!("Base58 checked address decoding failure on {} {}", s.clone(), e.to_string()),
            )
        })?;
        Ok(address_vec.into())
    }
    pub fn render_string(&self) -> Result<String, ErrorInfo> {
        let result = self.address.safe_bytes()?;
        Ok(address_to_str(&result))
    }
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Address, ErrorInfo> {
        Ok(address_data(bytes).expect("works"))
        // TODO: move that func here and leave alias there
    }

    pub fn from_public(pk: &PublicKey) -> Result<Address, ErrorInfo> {
        Self::from_bytes(address_function(pk))
    }
}

pub fn address_function(pk: &PublicKey) -> Vec<u8> {
    return address_function_buf(pk.serialize().as_ref());
}

pub fn multi_address_function(pk: &Vec<PublicKey>) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();
    for pki in pk {
        v.extend(pki.serialize().as_ref())
    }
    return address_function_buf(&*v);
}

pub fn address_function_buf(buf: &[u8]) -> Vec<u8> {
    use bitcoin::hashes::{hash160, Hash};
    let mut hash_engine = hash160::Hash::engine();
    hash_engine.write_all(buf).unwrap();
    let res = hash160::Hash::from_engine(hash_engine).clone().to_vec();
    return res;
}

pub fn str_to_address(s: String) -> Vec<u8> {
    return base58::from_check(&s[3..]).unwrap();
}

pub fn address_to_str(a: &Vec<u8>) -> String {
    let mut b = base58::check_encode_slice(&*a);
    b.insert_str(0, "rg1");
    return b;
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

pub fn address(pk: &PublicKey) -> Vec<u8> {
    return address_function(pk);
}

pub fn multi_address(public_key: &Vec<PublicKey>) -> Vec<u8> {
    return multi_address_function(public_key);
}

pub fn address_data(address: Vec<u8>) -> Option<Address> {
    Some(Address {
        address: bytes_data(address),
        address_type: Some(AddressType::StandardKeyhash as i32),
    })
}


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