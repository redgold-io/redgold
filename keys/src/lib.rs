use std::str::FromStr;
use bitcoin::secp256k1::Secp256k1;
use redgold_schema::{ErrorInfoContext, RgResult, SafeBytesAccess, structs};
use redgold_schema::structs::{Address, Hash};
use crate::util::{dhash_str, dhash_vec};
use crate::util::keys::ToPublicKeyFromLib;

pub mod proof_support;
pub mod request_support;
pub mod transaction_support;
pub mod util;

use crate::util::mnemonic_words::generate_key_i;

pub struct TestConstants {
    pub secret: bitcoin::secp256k1::SecretKey,
    pub public: bitcoin::secp256k1::PublicKey,
    pub public_peer_id: Vec<u8>,
    pub secret2: bitcoin::secp256k1::SecretKey,
    pub public2: bitcoin::secp256k1::PublicKey,
    pub hash: [u8; 32],
    pub hash_vec: Vec<u8>,
    pub addr: Vec<u8>,
    pub addr2: Vec<u8>,
    pub peer_ids: Vec<Vec<u8>>,
    pub peer_trusts: Vec<f64>,
    pub address_1: Address,
    pub rhash_1: Hash,
    pub rhash_2: Hash,
    pub words: String
}

impl TestConstants {
    pub fn key_pair(&self) -> KeyPair {
        KeyPair {
            secret_key: self.secret,
            public_key: self.public,
        }
    }

    pub fn new() -> TestConstants {
        let (secret, public) = crate::util::mnemonic_words::generate_key();
        let (secret2, public2) = generate_key_i(1);
        let hash = crate::util::dhash_str("asdf");
        let hash_vec = hash.to_vec();
        let addr = Address::from_struct_public(&public.to_struct_public_key()).expect("").address.safe_bytes().expect("");
        let addr2 = Address::from_struct_public(&public2.to_struct_public_key()).expect("").address.safe_bytes().expect("");
        let mut peer_ids: Vec<Vec<u8>> = Vec::new();
        let mut peer_trusts: Vec<f64> = Vec::new();

        for i in 0..10 {
            peer_ids.push(dhash_str(&i.to_string()).to_vec());
            peer_trusts.push((i as f64) / 10f64);
        }

        let public_peer_id = dhash_vec(&dhash_vec(&public.serialize().to_vec()).to_vec()).to_vec();

        return TestConstants {
            secret,
            public,
            public_peer_id,
            secret2,
            public2,
            hash,
            hash_vec,
            addr: addr.clone(),
            addr2,
            peer_ids,
            peer_trusts,
            address_1: addr.into(),
            rhash_1: Hash::from_string_calculate("asdf"),
            rhash_2: Hash::from_string_calculate("asdf2"),
            words: "abuse lock pledge crowd pair become ridge alone target viable black plate ripple sad tape victory blood river gloom air crash invite volcano release".to_string(),
        };
    }
}

#[derive(Clone, Copy)]
pub struct KeyPair {
    pub secret_key: bitcoin::secp256k1::SecretKey,
    pub public_key: bitcoin::secp256k1::PublicKey,
}

impl KeyPair {
    pub fn new(
        secret_key: &bitcoin::secp256k1::SecretKey,
        public_key: &bitcoin::secp256k1::PublicKey,
    ) -> Self {
        return Self {
            secret_key: *secret_key,
            public_key: *public_key,
        };
    }

    pub fn address(&self) -> Vec<u8> {
        Address::from_struct_public(&self.public_key.to_struct_public_key())
            .expect("").address.safe_bytes().expect("")
    }

    pub fn address_typed(&self) -> Address {
        self.public_key.to_struct_public_key().address().expect("")
    }

    pub fn public_key_vec(&self) -> Vec<u8> {
        self.public_key.serialize().to_vec()
    }

    pub fn public_key(&self) -> structs::PublicKey {
        self.public_key.to_struct_public_key()
    }

    pub fn from_private_hex(hex: String) -> RgResult<Self> {
        let secret_key = bitcoin::secp256k1::SecretKey::from_str(&*hex)
            .error_info("Unable to parse private key hex")?;
        let public_key = bitcoin::secp256k1::PublicKey::from_secret_key(&Secp256k1::new(), &secret_key);
        return Ok(Self {
            secret_key,
            public_key,
        });
    }
}
