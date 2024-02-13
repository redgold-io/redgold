use std::str::FromStr;
use bitcoin::{Address, Network};
use redgold_schema::structs::{AddressType, ErrorInfo, NetworkEnvironment, PublicKey};
use bitcoin::util::key;
use hex::ToHex;
// use web3::types::H160;
use redgold_schema::{ErrorInfoContext, SafeBytesAccess, structs};
use sha3::{Digest, Keccak256};
use crate::util::ToPublicKey;

pub trait ToBitcoinAddress {
    fn to_bitcoin_address(&self, network: &NetworkEnvironment) -> Result<String, ErrorInfo>;
}

pub trait ToEthereumAddress {
    fn to_ethereum_address(&self) -> Result<String, ErrorInfo>;
}


impl ToBitcoinAddress for PublicKey {
    fn to_bitcoin_address(&self, network: &NetworkEnvironment) -> Result<String, ErrorInfo> {

        let pk = &key::PublicKey::from_slice(&self.bytes()?).error_info("public key conversion")?;
        let network1 = if network == &NetworkEnvironment::Main {
            Network::Bitcoin
        } else {
            Network::Testnet
        };
        let address = Address::p2wpkh(pk, network1);
        Ok(address.to_string())
    }

}

impl ToBitcoinAddress for structs::Address {
    fn to_bitcoin_address(&self, network: &NetworkEnvironment) -> Result<String, ErrorInfo> {
        if self.is_bitcoin() {
            self.render_string()
        } else {
            Err(ErrorInfo::new("Address is not a bitcoin address"))
        }
    }

}

impl ToEthereumAddress for PublicKey {
    fn to_ethereum_address(&self) -> Result<String, ErrorInfo> {
        // ETH address requires uncompressed public key
        let data = self.to_lib_public_key()?.serialize_uncompressed().to_vec()[1..].to_vec();
        // Verify if this is correct.

        let mut hasher = Keccak256::new();
        hasher.update(data);
        let result = hasher.finalize().to_vec();
        let vec = result[12..].to_vec();
        let string = get_checksum_address(hex::encode(vec));
        Ok(string)
    }


}
// https://github.com/xenowits/eth-address/blob/main/src/address.rs
// Inspired from https://github.com/miguelmota/rust-eth-checksum
pub fn get_checksum_address(a: String) -> String {
    let addr = a.trim_start_matches("0x").to_lowercase();
    let address_hash = {
        let mut hasher = Keccak256::new();
        let x = addr.as_bytes();
        hasher.update(x);
        hex::encode(hasher.finalize().to_vec())
    };
    addr
    .char_indices()
    .fold(String::from("0x"), |mut acc, (index, address_char)| {
        // this cannot fail since it's Keccak256 hashed
        let n = u16::from_str_radix(&address_hash[index..index + 1], 16).unwrap();

        if n > 7 {
            // make char uppercase if ith character is 9..f
            acc.push_str(&address_char.to_uppercase().to_string())
        } else {
            // already lowercased
            acc.push(address_char)
        }

        acc
    })
}