use std::str::FromStr;
use bdk::bitcoin::{Network, PrivateKey, XpubIdentifier};
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
use redgold_schema::{error_info, ErrorInfoContext, RgResult, SafeBytesAccess, SafeOption, structs};
use redgold_schema::constants::{default_node_internal_derivation_path, REDGOLD_KEY_DERIVATION_PATH};
use redgold_schema::structs::{Hash, NetworkEnvironment, PeerId};
use crate::address_external::{ToBitcoinAddress, ToEthereumAddress};
use crate::KeyPair;
use crate::util::btc_wallet::{SingleKeyBitcoinWallet, struct_public_to_address};
use crate::util::mnemonic_words::MnemonicWords;

#[derive(Clone, Serialize, Deserialize)]
pub struct WordsPass {
    pub words: String,
    pub passphrase: Option<String>,
}

trait MnemonicSupport {

}

#[derive(Clone, Serialize, Deserialize)]
pub struct WordsPassBtcMessageAccountMetadata {
    derivation_path: String,
    account: u32,
    rdg_address: String,
    pub xpub: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WordsPassMetadata {
    checksum: String,
    checksum_words: String,
    btc_84h_0h_0h_0_0_address: String,
    btc_84h_0h_0h_0_0_xpub: String,
    eth_44h_60h_0h_0_0_address: String,
    eth_44h_60h_0h_0_0_xpub: String,
    rdg_44h_16180h_0h_0_0_address: String,
    rdg_44h_16180h_0h_0_0_xpub: String,
    rdg_btc_message_account_metadata: Vec<WordsPassBtcMessageAccountMetadata>,
    executable_checksum: String
}

impl WordsPassMetadata {
    pub fn with_exe_checksum(&mut self, sum: impl Into<String>) -> &mut WordsPassMetadata {
        self.executable_checksum = sum.into();
        self
    }
}

impl WordsPass {

    pub fn metadata(&self) -> RgResult<WordsPassMetadata> {
        let mut res = vec![];
        // Default spending keys
        let mut sequence = (0..10).collect_vec().iter().map(|i| 50 + i).collect_vec();
        // Default peer ids
        sequence.extend((0..10).collect_vec().iter().map(|i| 99 - i).collect_vec());
        for account in sequence {
            let path = format!("m/44'/0'/{}'/0/0", account);
            let xpub_path = format!("m/44'/0'/{}'", account);
            res.push(WordsPassBtcMessageAccountMetadata {
                derivation_path: path.clone(),
                account: 0,
                rdg_address: self.public_at(path)?.address()?.render_string()?,
                xpub: self.xpub(xpub_path)?.to_hex(),
            });
        }
        Ok(WordsPassMetadata {
            checksum: self.checksum()?,
            checksum_words: self.checksum_words()?,
            btc_84h_0h_0h_0_0_address: self.public_at("m/84'/0'/0'/0/0")?.to_bitcoin_address()?,
            btc_84h_0h_0h_0_0_xpub: self.xpub("m/84'/0'/0'")?.to_hex(),
            eth_44h_60h_0h_0_0_address: self.public_at("m/44'/60'/0'/0/0")?.to_ethereum_address()?,
            eth_44h_60h_0h_0_0_xpub: self.xpub("m/44'/60'/0'")?.to_hex(),
            rdg_44h_16180h_0h_0_0_address: self.public_at("m/44'/16180'/0'/0/0")?.address()?.render_string()?,
            rdg_44h_16180h_0h_0_0_xpub: self.xpub("m/44'/16180'/0'")?.to_hex(),
            rdg_btc_message_account_metadata: res,
            executable_checksum: "".to_string(),
        })
    }

    pub fn default_rg_path(account: usize) -> String {
        default_node_internal_derivation_path(account as i64)
    }

    pub fn kp_rg_account(&self, account: usize) -> RgResult<KeyPair> {
        self.keypair_at(Self::default_rg_path(account))
    }

    pub fn default_kp(&self) -> RgResult<KeyPair> {
        self.kp_rg_account(0)
    }

    pub fn default_pid_kp(&self) -> RgResult<KeyPair> {
        self.kp_rg_account(1)
    }

    pub fn checksum(&self) -> RgResult<String> {
        Hash::new_checksum(&self.seed()?.to_vec())
    }

    pub fn checksum_words(&self) -> RgResult<String> {
        let mut s2 = self.clone();
        s2.passphrase = None;
        let s = s2.seed()?.to_vec();
        Hash::new_checksum(&s)
    }

    pub fn seed(&self) -> RgResult<[u8; 64]> {
        Ok(self.mnemonic()?.to_seed(self.passphrase.clone().unwrap_or("".to_string())))
    }

    pub fn hash_derive_words(&self, concat_nonce: impl Into<String>) -> RgResult<Self> {
        let mut vec = self.seed()?.to_vec();
        vec.extend(concat_nonce.into().as_bytes());
        let entropy = structs::Hash::digest(vec).safe_bytes()?;
        let m = Mnemonic::from_entropy(&*entropy).error_info("Failed to derive mnemonic from entropy")?;
        Ok(Self {
            words: m.to_string(),
            passphrase: None
        })
    }

    pub fn new(words: impl Into<String>, passphrase: Option<String>) -> Self {
        Self {
            words: words.into(),
            passphrase,
        }
    }
    pub fn words(words: String) -> Self {
        Self {
            words,
            passphrase: None
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

    pub fn xpub(&self, path: impl Into<String>) -> RgResult<XpubIdentifier> {
        Ok(self.key_from_path_str(path.into())?.identifier(&Secp256k1::new()))
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

    pub fn keypair_at(&self, path: String) -> RgResult<KeyPair> {
        KeyPair::from_private_hex(self.private_at(path)?)
    }

    pub fn public_at(&self, path: impl Into<String>) -> RgResult<structs::PublicKey> {
        let key = self.key_from_path_str(path.into())?;
        let vec = key.private_key.public_key(&Secp256k1::new()).serialize().to_vec();
        Ok(structs::PublicKey::from_bytes(vec))
    }

    pub fn default_peer_id(&self) -> RgResult<PeerId> {
        let pk = self.public_at(default_node_internal_derivation_path(1))?;
        let pid = PeerId::from_pk(pk);
        Ok(pid)
    }
    pub fn default_public_key(&self) -> RgResult<structs::PublicKey> {
        let pk = self.public_at(default_node_internal_derivation_path(0))?;
        Ok(pk)
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

#[test]
pub fn generate_xpub() {
    let w = WordsPass::generate().expect("words");
    println!("{}", w.words.clone());
    w.public_at("m/44'/0'/0'".to_string()).expect("private key");
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
    // let test_seed_no_p = mnemonic.to_seed(None);
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