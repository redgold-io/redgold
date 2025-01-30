#![allow(unused_imports)]

use crate::util::keys::ToPublicKeyFromLib;
use crate::util::mnemonic_support::MnemonicSupport;
use bdk::bitcoin::hashes::hex::ToHex;
use bdk::bitcoin::secp256k1::Secp256k1;
use redgold_schema::keys::words_pass::WordsPass;
use redgold_schema::structs::{Address, Hash};
use redgold_schema::{structs, ErrorInfoContext, RgResult};
use std::str::FromStr;

pub mod proof_support;
pub mod request_support;
pub mod transaction_support;
pub mod util;
pub mod debug;
pub mod xpub_wrapper;
pub mod address_external;
pub mod address_support;
pub mod hw_wallet_wrapper;
pub mod tx_proof_validate;
pub mod public_key_parse_support;
pub mod external_tx_support;
pub mod solana;
pub mod monero;
pub mod gpg;
pub mod word_pass_support;
pub mod eth;
pub mod btc;

pub struct TestConstants {
    pub secret: bdk::bitcoin::secp256k1::SecretKey,
    pub public: bdk::bitcoin::secp256k1::PublicKey,
    pub secret2: bdk::bitcoin::secp256k1::SecretKey,
    pub public2: bdk::bitcoin::secp256k1::PublicKey,
    pub hash_vec: Vec<u8>,
    pub address_1: Address,
    pub rhash_1: Hash,
    pub rhash_2: Hash,
    pub words: String,
    pub words_pass: WordsPass,
}

impl TestConstants {

    pub fn dev_ci_kp_path() -> String {
        "m/84'/0'/0'/0/0".to_string()
    }


    pub fn test_words() -> Option<String> {
        std::env::var("REDGOLD_TEST_WORDS").ok()
    }

    pub fn test_words_pass() -> Option<WordsPass> {
        Self::test_words().map(|w| WordsPass::new(w, None))
    }

    pub fn key_pair(&self) -> KeyPair {
        KeyPair {
            secret_key: self.secret,
            public_key: self.public,
        }
    }

    pub fn new() -> TestConstants {
        let result = WordsPass::from_str_hashed("test_constants");
        let kp_default = result.default_kp().expect("");
        let (secret, public) = (kp_default.secret_key, kp_default.public_key);
        let kp2 = result.keypair_at_change(1).expect("");
        let (secret2, public2) = (kp2.secret_key, kp2.public_key);
        let hash_vec = Hash::from_string_calculate("asdf1").vec();
        let addr = Address::from_struct_public(&public.to_struct_public_key()).expect("");
        // let addr2 = Address::from_struct_public(&public2.to_struct_public_key()).expect("")
        return TestConstants {
            secret,
            public,
            secret2,
            public2,
            hash_vec,
            address_1: addr,
            rhash_1: Hash::from_string_calculate("asdf"),
            rhash_2: Hash::from_string_calculate("asdf2"),
            words: "abuse lock pledge crowd pair become ridge alone target viable black plate ripple sad tape victory blood river gloom air crash invite volcano release".to_string(),
            words_pass: result
        };
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct KeyPair {
    pub secret_key: bdk::bitcoin::secp256k1::SecretKey,
    pub public_key: bdk::bitcoin::secp256k1::PublicKey,
}


impl KeyPair {
    pub fn new(
        secret_key: &bdk::bitcoin::secp256k1::SecretKey,
        public_key: &bdk::bitcoin::secp256k1::PublicKey,
    ) -> Self {
        return Self {
            secret_key: *secret_key,
            public_key: *public_key,
        };
    }

    pub fn address_typed(&self) -> Address {
        self.public_key().address().expect("")
    }

    pub fn public_key(&self) -> structs::PublicKey {
        self.public_key.to_struct_public_key()
    }

    pub fn from_private_hex(hex: String) -> RgResult<Self> {
        let secret_key = bdk::bitcoin::secp256k1::SecretKey::from_str(&*hex)
            .error_info("Unable to parse private key hex")?;
        let public_key = bdk::bitcoin::secp256k1::PublicKey::from_secret_key(&Secp256k1::new(), &secret_key);
        return Ok(Self {
            secret_key,
            public_key,
        });
    }
    pub fn to_private_hex(&self) -> String {
        self.secret_key.secret_bytes().to_hex()
    }
}

#[test]
fn debug_addr() {
    let tc = TestConstants::new();
    let kp = tc.key_pair();
    let addr = kp.address_typed();
    println!("addr: {:?}", addr);
    println!("addr: {}", addr.render_string().expect(""));
    println!("addr: {}", addr.raw_bytes().expect("").to_hex());
}