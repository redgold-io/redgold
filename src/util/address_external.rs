use std::str::FromStr;
use bitcoin::{Address, Network};
use redgold_schema::public_key::ToPublicKey;
use redgold_schema::structs::{ErrorInfo, PublicKey};
use bitcoin::util::key;
use crypto::digest::Digest;
use crypto::sha3::Sha3;
use crypto::sha3::Sha3Mode::Keccak256;
use redgold_schema::ErrorInfoContext;

pub trait ToBitcoinAddress {
    fn to_bitcoin_address(&self) -> Result<String, ErrorInfo>;
}

pub trait ToEthereumAddress {
    fn to_ethereum_address(&self) -> Result<String, ErrorInfo>;
}


impl ToBitcoinAddress for PublicKey {
    fn to_bitcoin_address(&self) -> Result<String, ErrorInfo> {
        let pk = &key::PublicKey::from_slice(&self.bytes()?).error_info("public key conversion")?;
        let address = Address::p2wpkh(pk, Network::Bitcoin);
        Ok(address.to_string())
    }
}

impl ToEthereumAddress for PublicKey {
    fn to_ethereum_address(&self) -> Result<String, ErrorInfo> {
        // Verify if this is correct.
        let mut sha3 = Sha3::keccak256();
        let b = &self.bytes()?;
        sha3.input(b);
        let mut res = [0u8; 32];
        sha3.result(&mut res);
        Ok(hex::encode(res[12..].to_vec()))
    }
}