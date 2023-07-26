use std::str::FromStr;
use bdk::bitcoin::Network;
use bdk::bitcoin::secp256k1::Secp256k1;
use bdk::bitcoin::util::bip32::DerivationPath;
use bdk::keys::bip39::{Mnemonic, Language};
use bdk::keys::{DerivableKey, ExtendedKey};
use bitcoin::hashes::hex::ToHex;
use bitcoin_wallet::account::MasterKeyEntropy;
use serde::Serialize;
use crate::TestConstants;
use crate::util::mnemonic_words::MnemonicWords;

#[test]
pub fn test() {

    let words = bitcoin_wallet::mnemonic::Mnemonic::new_random(MasterKeyEntropy::Double)
        .unwrap().to_string();
    println!("{}", words.clone());

    let mnemonic1 = MnemonicWords::from_mnemonic_words(words.clone().as_str(), Some("test".to_string()));
    let seed1 = mnemonic1.seed.0.clone();

    let mnemonic = Mnemonic::parse_in(
        Language::English,
        words.clone(),
    ).unwrap();
    let seed = mnemonic.to_seed("test");
    assert_eq!(seed1.clone(), seed.clone().to_vec());
    let xkey: ExtendedKey =
        (mnemonic, Some("test".to_string()))
            .into_extended_key().unwrap();
    let xprv = xkey.into_xprv(Network::Bitcoin).unwrap();
    let k1 = Secp256k1::new();
    let path = "m/84'/0'/0'/0/0";
    let dp = DerivationPath::from_str(path).unwrap();
    let key1 = xprv.derive_priv(&k1, &dp).unwrap();
    let pkhex = hex::encode(key1.private_key.secret_bytes().to_vec());
    println!("Pkhex {}", pkhex.clone());

    let pkhex2 = mnemonic1.key_from_path_str(path.to_string()).0.to_hex();
    println!("Pkhex2 {}", pkhex2.clone());
    assert_eq!(pkhex, pkhex2);
    ()

}