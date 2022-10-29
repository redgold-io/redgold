use std::str::FromStr as frmstr;

use bitcoin::{
    network::constants::Network,
    secp256k1,
    secp256k1::Secp256k1,
    util::bip32::{DerivationPath, ExtendedPrivKey, ExtendedPubKey},
};
// #[cfg(test)]
// use bitcoin::hashes::core::str::FromStr;
use bitcoin::hashes::hex::ToHex;
use bitcoin::secp256k1::{PublicKey, SecretKey};
use bitcoin::util::bip32::ChildNumber;
use bitcoin_wallet::account::Seed;
use bitcoin_wallet::mnemonic::Mnemonic;
use hdpath::{Purpose, StandardHDPath};

use crate::constants::REDGOLD_KEY_DERIVATION_PATH;
use crate::KeyPair;
use crate::util::mnemonic_builder;

// use libp2p::identity::{secp256k1, Keypair};

pub const REDGOLD_SLIP_IDX: u32 = 9000;
pub const STANDARD_TEST_PHRASE: &str = "somelongpasswordwithhighentropygoeshere";
// pub const PEER_ID_ACCOUNT: u32 = 1;

fn default_cursor() -> StandardHDPath {
    let cursor = StandardHDPath::new(Purpose::Pubkey, REDGOLD_SLIP_IDX, 0, 0, 0);
    return cursor;
}

// why must CI hate this
fn from_cn(value: &StandardHDPath) -> Vec<ChildNumber> {
    let result = [
        ChildNumber::from_hardened_idx(value.purpose().as_value().as_number())
            .expect("Purpose is not Hardened"),
        ChildNumber::from_hardened_idx(value.coin_type()).expect("Coin Type is not Hardened"),
        ChildNumber::from_hardened_idx(value.account()).expect("Account is not Hardened"),
        ChildNumber::from_normal_idx(value.change()).expect("Change is Hardened"),
        ChildNumber::from_normal_idx(value.index()).expect("Index is Hardened"),
    ];
    return result.to_vec();
}

pub fn get_pk(seed: &[u8], hd_path: &StandardHDPath) -> ExtendedPrivKey {
    let secp = Secp256k1::new();
    // this crazy direct reference is required to fix CI for some reason?
    let path = DerivationPath::from(from_cn(&hd_path));
    //let path: DerivationPath = std::convert::From::<StandardHDPath>::from(hd_path.clone());
    ExtendedPrivKey::new_master(Network::Bitcoin, seed)
        // we convert HD Path to bitcoin lib format (DerivationPath)
        // Why does this line below fail on CI but not locally?
        //        .and_then(|k| k.derive_priv(&secp, &DerivationPath::from(hd_path)))
        .and_then(|k| k.derive_priv(&secp, &path))
        .unwrap()
}
//
// pub fn get_epk(seed: &[u8], network: Network) -> ExtendedPrivKey {
//     let secp = Secp256k1::new();
//     // this crazy direct reference is required to fix CI for some reason?
//     let path = DerivationPath::from(from_cn(&hd_path));
//     //let path: DerivationPath = std::convert::From::<StandardHDPath>::from(hd_path.clone());
//     ExtendedPrivKey::new_master(network, seed)
//         // we convert HD Path to bitcoin lib format (DerivationPath)
//         // Why does this line below fail on CI but not locally?
//         //        .and_then(|k| k.derive_priv(&secp, &DerivationPath::from(hd_path)))
//         .and_then(|k| k.derive_priv(&secp, &path))
//         .unwrap()
// }

#[derive(Clone)]
pub struct Wallet {
    pub seed: Seed,
    cursor: StandardHDPath,
}

impl Wallet {
    pub fn from_phrase(s: &str) -> Self {
        let m = mnemonic_builder::from_str(s);

        return Wallet {
            seed: m.to_seed(None),
            cursor: default_cursor(),
        };
    } // let hd_path = StandardHDPath::from_str("m/44'/0'/0'/0/0").unwrap();
    #[allow(dead_code, unused_assignments)]
    pub fn from_mnemonic(s: &str, passphrase: Option<String>) -> Self {
        let m = Mnemonic::from_str(s).unwrap();

        let mut option: Option<&str> = None;
        let mut value: String = "".to_string();
        match passphrase.clone() {
            Some(p) => {
                value = p;
                option = Some(&*value)
            }
            None => {}
        }
        return Wallet {
            seed: m.to_seed(option.clone()),
            cursor: default_cursor(),
        };
    } // let hd_path = StandardHDPath::from_str("m/44'/0'/0'/0/0").unwrap();

    pub fn default() -> Self {
        return Wallet::from_phrase(STANDARD_TEST_PHRASE);
    }

    pub fn key(&self, hd_path: &StandardHDPath) -> (SecretKey, PublicKey) {
        let key = get_pk(&self.seed.0, hd_path);
        let pk = key.private_key.key;
        let pub_key = Wallet::get_public_key(&key);
        return (pk, pub_key);
    }

    pub fn keypair(&self, hd_path: &StandardHDPath) -> KeyPair {
        let (pk, pub_key) = self.key(hd_path);
        return KeyPair::new(&pk, &pub_key);
    }

    pub fn key_from_path_str(&self, hd_path: String) -> (SecretKey, PublicKey) {
        let hd_path = StandardHDPath::from_str(&*hd_path).unwrap();
        let key = get_pk(&self.seed.0, &hd_path);
        let pk = key.private_key.key;
        let pub_key = Wallet::get_public_key(&key);
        return (pk, pub_key);
    }

    pub fn keypair_from_path_str(&self, hd_path: String) -> KeyPair {
        let hd_path = StandardHDPath::from_str(&*hd_path).unwrap();
        let key = get_pk(&self.seed.0, &hd_path);
        let pk = key.private_key.key;
        let pub_key = Wallet::get_public_key(&key);
        return KeyPair::new(&pk, &pub_key);
    }

    pub fn get_public_key(key: &ExtendedPrivKey) -> PublicKey {
        let secp = Secp256k1::new();
        let pub_key = ExtendedPubKey::from_private(&secp, &key).public_key.key;
        pub_key
    }

    pub fn active_key(&self) -> (SecretKey, PublicKey) {
        return self.key(&self.cursor);
    }

    pub fn active_keypair(&self) -> KeyPair {
        let x = self.active_key();
        return KeyPair::new(&x.0, &x.1);
    }


    pub fn transport_key(&self) -> KeyPair {
        let x = self.active_key();
        return KeyPair::new(&x.0, &x.1);
    }

    pub fn next_key(&mut self) -> (SecretKey, PublicKey) {
        self.cursor = StandardHDPath::new(
            Purpose::Pubkey,
            self.cursor.coin_type(),
            self.cursor.account(),
            self.cursor.change(),
            self.cursor.index() + 1,
        );
        return self.active_key();
    }

    pub fn key_at(&self, index: usize) -> KeyPair {
        let cursor = StandardHDPath::new(
            Purpose::Witness,
            REDGOLD_KEY_DERIVATION_PATH as u32,
            0,
            0,
            index as u32,
        );
        return self.keypair(&cursor);
    }
}

// Change to impl?
//
// #[test]
// fn test_serialization_translation() {
//     let (s, _) = generate_key();
//     let h = s.to_hex();
//     println!("{}", h);
//     let s2 = SecretKey::from_str(&*h).unwrap();
//     let hex_dec = hex::decode(h).unwrap();
//     let s3 = SecretKey::from_slice(&*hex_dec).unwrap();
//     assert_eq!(s2, s);
//     assert_eq!(s3, s);
//     // libsecp256k1::SecretKey
//     let hex_dec2 = hex_dec.clone();
//     //Keypair::secp256k1_from_der(&mut *hex_dec2).unwrap();
//     let s4 = secp256k1::SecretKey::from_bytes(hex_dec2).unwrap();
//     // assert_eq!(s4, s);
//     let kp1 = secp256k1::Keypair::from(s4);
//     let _kp2 = Keypair::Secp256k1(kp1);
//     //assert_eq!(kp2.public()., p);
// }

pub fn generate_keys(range: u16) -> Vec<(SecretKey, PublicKey)> {
    let mut wallet = Wallet::default();
    let mut keys: Vec<(SecretKey, PublicKey)> = Vec::new();
    for _ in 0..range {
        keys.push(wallet.next_key());
    }
    return keys;
}

pub fn generate_key() -> (SecretKey, PublicKey) {
    return *generate_keys(1).get(0).unwrap();
}

pub fn generate_key_i(offset: usize) -> (SecretKey, PublicKey) {
    return *generate_keys((offset + 1) as u16).get(offset).unwrap();
}

#[test]
fn test_next_key() {
    let mut wallet = Wallet::default();
    let key1 = wallet.next_key();
    let key2 = wallet.next_key();
    println!("key1: {:?}", key1);
    println!("key2: {:?}", key2);
    assert_ne!(key1.0.to_string(), key2.0.to_string());
}

#[test]
fn test_hex_ser() {
    let mut wallet = Wallet::default();
    let (secret, pubkey) = wallet.next_key();
    // println!("secret: {:?} pubkey: {:?}", secret, pubkey);
    let hex = secret.to_hex();
    let hexp = pubkey.to_hex();

    let decoded = SecretKey::from_str(&*hex).unwrap();
    let decodedp = PublicKey::from_str(&*hexp).unwrap();
    assert_eq!(secret, decoded);
    assert_eq!(secret.to_string(), decoded.to_string());
    assert_eq!(pubkey, decodedp);
    assert_eq!(pubkey.to_string(), decodedp.to_string());
}
