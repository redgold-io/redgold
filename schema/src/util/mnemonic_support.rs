use std::str::FromStr;
use bdk::bitcoin::{Network, PrivateKey};
use bdk::bitcoin::secp256k1::{rand, Secp256k1};
use bdk::bitcoin::secp256k1::rand::RngCore;
use bdk::bitcoin::util::bip32::{DerivationPath, ExtendedPrivKey};
use bdk::keys::bip39::{Mnemonic, Language};
use bdk::keys::{DerivableKey, ExtendedKey, GeneratableKey, GeneratedKey};
use bdk::keys::bip39::WordCount::Words24;
use bdk::miniscript::miniscript;
use bitcoin::hashes::hex::ToHex;
use bitcoin_wallet::account::MasterKeyEntropy;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use crate::{error_info, ErrorInfoContext, KeyPair, RgResult, SafeOption, structs, TestConstants};
use crate::structs::NetworkEnvironment;
use crate::util::btc_wallet::{SingleKeyBitcoinWallet, struct_public_to_address};
use crate::util::mnemonic_words::MnemonicWords;

#[derive(Clone, Serialize, Deserialize)]
struct WordsPass {
    words: String,
    passphrase: Option<String>,
}

impl WordsPass {
    pub fn new(words: String, passphrase: Option<String>) -> Self {
        Self {
            words,
            passphrase,
        }
    }
    pub fn generate() -> RgResult<Self> {
        let mut rng = rand::thread_rng();
        let mut entropy = [0u8; 32];
        rng.fill_bytes(&mut entropy);
        let wc = Words24;
        let language = Language::English;
        let pair = (wc, language);
        let generated: RgResult<GeneratedKey<_, miniscript::Segwitv0>> =
            Mnemonic::generate_with_entropy(pair, entropy).map_err(
                |e| error_info(format!("Failed to generate mnemonic: {}",
                e.map(|e| e.to_string()).unwrap_or("".to_string()))));
        Ok(Self {
            words: generated?.to_string(),
            passphrase: None,
        })
    }

    pub fn mnemonic(&self) -> RgResult<Mnemonic> {
        Mnemonic::parse_in(
            Language::English,
            self.words.clone(),
        ).map_err(|e| error_info(format!("Failed to parse mnemonic: {}",
        e.to_string())))
    }

    pub fn pair(&self) -> RgResult<(Mnemonic, Option<String>)> {
        Ok((self.mnemonic()?, self.passphrase.clone()))
    }

    pub fn extended_key(&self) -> RgResult<ExtendedKey> {
        self.pair()?.into_extended_key()
            .map_err(|e| error_info(format!("Failed to generate extended key: {}", e.to_string())))
    }

    pub fn xprv(&self) -> RgResult<ExtendedPrivKey> {
        self.extended_key()?.into_xprv(Network::Bitcoin)
            .safe_get_msg("Failed to generate xprv").cloned()
    }

    pub fn key_from_path_str(&self, path: String) -> RgResult<ExtendedPrivKey> {
        let dp = DerivationPath::from_str(path.as_str())
            .error_info("Failed to parse derivation path")?;
        Ok(self.xprv()?.derive_priv(&Secp256k1::new(), &dp)
            .error_info("Failed to derive private key")?)
    }

    pub fn private_at(&self, path: String) -> RgResult<String> {
        let key = self.key_from_path_str(path)?;
        let pkhex = hex::encode(key.private_key.secret_bytes().to_vec());
        Ok(pkhex)
    }

    pub fn public_at(&self, path: String) -> RgResult<structs::PublicKey> {
        let key = self.key_from_path_str(path)?;
        let vec = key.private_key.public_key(&Secp256k1::new()).serialize().to_vec();
        Ok(structs::PublicKey::from_bytes(vec))
    }

    /*
    let k1 = Secp256k1::new();
    let path = "m/84'/0'/0'/0/0";
    let dp = DerivationPath::from_str(path).unwrap();
    let key1 = xprv.derive_priv(&k1, &dp).unwrap();
    let pkhex = hex::encode(key1.private_key.secret_bytes().to_vec());
    println!("Pkhex {}", pkhex.clone());

    let pkhex2 = mnemonic1.key_from_path_str(path.to_string()).0.to_hex();
    println!("Pkhex2 {}", pkhex2.clone());
    assert_eq!(pkhex, pkhex2);
     */
}

#[test]
pub fn generate_test() {
    let w = WordsPass::generate().expect("words");
    println!("{}", w.words.clone());
    assert_eq!(24, w.words.split(" ").collect_vec().len());
}

pub fn test_pkey_hex() -> Option<String> {
    if let Some(w) = std::env::var("REDGOLD_TEST_WORDS").ok() {
        let w = WordsPass::new(w, None);
        let path = "m/84'/0'/0'/0/0";
        Some(w.private_at(path.to_string()).expect("private key"))
    } else {
        None
    }
}

pub fn test_pubk() -> Option<structs::PublicKey> {
    if let Some(w) = std::env::var("REDGOLD_TEST_WORDS").ok() {
        let w = WordsPass::new(w, None);
        let path = "m/84'/0'/0'/0/0";
        Some(w.public_at(path.to_string()).expect("private key"))
    } else {
        None
    }
}

#[test]
pub fn load_ci_kp() {
    if let Some(w) = std::env::var("REDGOLD_TEST_WORDS").ok() {
        let w = WordsPass::new(w, None);
        let path = "m/84'/0'/0'/0/0";
        let pk = w.public_at(path.to_string()).expect("private key");
        let w =
            SingleKeyBitcoinWallet::new_wallet(pk, NetworkEnvironment::Dev, true)
                .expect("w");
        let a = w.address().expect("a");
        // tb1qrxdzt6v9yuu567j52cmla4v9kler3wzj9swxy9
        println!("{a}");
    }
}


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