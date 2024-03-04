use crate::structs::{Address, AddressInfo, AddressType, Error, ErrorInfo, Hash, SupportedCurrency, UtxoEntry};
use crate::{bytes_data, error_info, ErrorInfoContext, from_hex, RgResult, SafeBytesAccess};
use crate::{error_message, structs};
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


    pub fn script_hash(input: impl AsRef<[u8]>) -> RgResult<Self> {
        let mut new = Self::from_bytes(Self::hash(input.as_ref()))?;
        new.address_type = AddressType::ScriptHash as i32;
        Ok(new)
    }

    // Maybe consider checking here to see if the address is valid?
    // Or before this actually, we should potentially not do 'into bytes'
    // but instead change this to a vec and decode it.
    pub fn from_bitcoin(address: &String) -> Address {
        Self {
            address: bytes_data(address.clone().into_bytes()),
            address_type: AddressType::BitcoinExternalString as i32,
            currency: Some(SupportedCurrency::Bitcoin as i32),
        }
    }
    pub fn from_eth(address: &String) -> Address {
        Self {
            address: bytes_data(address.clone().into_bytes()),
            address_type: AddressType::EthereumExternalString as i32,
            currency: Some(SupportedCurrency::Ethereum as i32),
        }
    }

    pub fn is_bitcoin(&self) -> bool {
        self.address_type == AddressType::BitcoinExternalString as i32
    }

    pub fn render_string(&self) -> Result<String, ErrorInfo> {
        let result = self.address.safe_bytes()?;
        if self.address_type == AddressType::BitcoinExternalString as i32 {
            return Ok(String::from_utf8(result).error_info("Unable to convert bitcoin address bytes to utf8 string")?);
        }
        Ok(Self::address_to_str(&result))
    }
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Address, ErrorInfo> {
        let addr = Self::new_raw(bytes);
        addr.verify_checksum()?;

        Ok(addr)
    }

    pub fn from_struct_public(pk: &structs::PublicKey) -> Result<Address, ErrorInfo> {
        Self::from_bytes(Self::hash(&pk.bytes.safe_bytes()?))
    }

    pub fn with_checksum(bytes: Vec<u8>) -> Vec<u8> {
        let checksum_bytes = Hash::digest(bytes.clone()).vec();
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

    pub fn address_data(address: Vec<u8>) -> Option<Address> {
        Some(Self::new_raw(address))
    }

    pub fn new_raw(address: Vec<u8>) -> Address {
        Address {
            address: bytes_data(address),
            address_type: AddressType::Sha3224ChecksumPublic as i32,
            currency: None,
        }
    }


    fn from_hex(p0: String) -> Result<Address, ErrorInfo> {
        let bytes = from_hex(p0)?;
        Address::from_bytes(bytes)
    }
}



impl AddressInfo {
    pub fn from_utxo_entries(address: Address, entries: Vec<UtxoEntry>) -> Self {
        let mut bal: i64 = 0;
        for r in &entries {
            if let Some(o) = &r.output {
                if let Some(d) = &o.data {
                    if let Some(a) = &d.amount {
                        bal += a.amount;
                    }
                }
            }
        }
        AddressInfo {
            address: Some(address.clone()),
            utxo_entries: entries,
            balance: bal,
            recent_transactions: vec![]
        }
    }
}