use std::str::FromStr;

use bdk::bitcoin::secp256k1::rand::RngCore;
use bdk::bitcoin::secp256k1::{rand, Secp256k1};
use bdk::bitcoin::util::bip32::{DerivationPath, ExtendedPrivKey, ExtendedPubKey};
use bdk::bitcoin::Network;
use bdk::keys::bip39::WordCount::Words24;
use bdk::keys::bip39::{Language, Mnemonic};
use bdk::keys::{DerivableKey, ExtendedKey, GeneratableKey, GeneratedKey};
use bdk::miniscript::miniscript;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use redgold_schema::conf::local_stored_state::{AccountKeySource, XPubLikeRequestType};
use redgold_schema::constants::{default_node_internal_derivation_path, redgold_keypair_change_path};
use redgold_schema::keys::words_pass::WordsPass;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{Address, Hash, NetworkEnvironment, PeerId};
use redgold_schema::{error_info, structs, ErrorInfoContext, RgResult, SafeOption};

use crate::address_external::{ToBitcoinAddress, ToEthereumAddress};
use crate::btc::btc_wallet::SingleKeyBitcoinWallet;
use crate::proof_support::PublicKeySupport;
use crate::xpub_wrapper::ValidateDerivationPath;
use crate::KeyPair;
use crate::monero::key_derive::MoneroSeedBytes;
use crate::solana::derive_solana::SolanaWordPassExt;

pub trait MnemonicSupport {
    fn metadata(&self) -> RgResult<WordsPassMetadata>;
    fn default_rg_path(account: usize) -> String;
    fn kp_rg_account(&self, account: usize) -> RgResult<KeyPair>;
    fn default_kp(&self) -> RgResult<KeyPair>;
    fn default_xpub(&self) -> RgResult<String>;
    fn named_xpub(&self, key_name: impl Into<String>, skip_persist: bool, n: &NetworkEnvironment) -> RgResult<AccountKeySource>;
    fn default_pid_kp(&self) -> RgResult<KeyPair>;
    fn checksum(&self) -> RgResult<String>;
    fn checksum_words(&self) -> RgResult<String>;
    fn seed(&self) -> RgResult<[u8; 64]>;
    fn hash_derive_words(&self, concat_nonce: impl Into<String>) -> RgResult<Self> where Self: Sized;
    fn new_validated(words: impl Into<String>, passphrase: Option<String>) -> RgResult<Self> where Self: Sized;
    fn words(words: String) -> Self  where Self: Sized;
    fn generate() -> RgResult<Self>  where Self: Sized;
    fn from_bytes(bytes: &[u8]) -> RgResult<Self> where Self: Sized;
    fn from_str_hashed(str: impl Into<String>) -> Self where Self: Sized;
    fn mnemonic(&self) -> RgResult<Mnemonic>;
    fn validate(&self) -> RgResult<&Self> where Self: Sized;
    fn pair(&self) -> RgResult<(Mnemonic, Option<String>)>;
    fn extended_key(&self) -> RgResult<ExtendedKey>;
    fn xprv(&self) -> RgResult<ExtendedPrivKey>;
    fn xpub(&self, path: impl Into<String>) -> RgResult<ExtendedPubKey>;
    fn derive_seed_at_path(&self, path: &str) -> RgResult<[u8; 32]>;
    fn xpub_str(&self, path: impl Into<String>) -> RgResult<String>;
    fn key_from_path_str(&self, path: impl Into<String>) -> RgResult<ExtendedPrivKey>;
    fn private_at(&self, path: impl Into<String>) -> RgResult<String>;
    fn keypair_at(&self, path: impl Into<String>) -> RgResult<KeyPair>;
    fn public_at(&self, path: impl Into<String>) -> RgResult<structs::PublicKey>;
    fn keypair_at_change(&self, change: impl Into<i64>) -> RgResult<KeyPair>;
    fn default_peer_id(&self) -> RgResult<PeerId>;
    fn default_public_key(&self) -> RgResult<structs::PublicKey>;
    fn test_words() -> Self;
    fn to_all_addresses_default(&self, net: &NetworkEnvironment) -> RgResult<Vec<Address>>;
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WordsPassBtcMessageAccountMetadata {
    pub derivation_path: String,
    pub account: u32,
    pub rdg_address: String,
    pub rdg_btc_main_address: String,
    pub rdg_btc_test_address: String,
    pub xpub: String,
    pub public_hex: String
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WordsPassMetadata {
    pub checksum: String,
    pub checksum_words: String,
    pub btc_84h_0h_0h_0_0_address: String,
    pub btc_84h_0h_0h_0_0_xpub: String,
    pub eth_44h_60h_0h_0_0_address: String,
    pub eth_44h_60h_0h_0_0_xpub: String,
    pub rdg_44h_16180h_0h_0_0_address: String,
    pub rdg_44h_16180h_0h_0_0_xpub: String,
    pub rdg_btc_message_account_metadata: Vec<WordsPassBtcMessageAccountMetadata>,
    pub executable_checksum: String
}

impl WordsPassMetadata {
    pub fn with_exe_checksum(&mut self, sum: impl Into<String>) -> &mut WordsPassMetadata {
        self.executable_checksum = sum.into();
        self
    }
}

impl MnemonicSupport for WordsPass {

    fn to_all_addresses_default(&self, net: &NetworkEnvironment) -> RgResult<Vec<Address>> {
        Ok(vec![
            self.default_public_key()?.address()?,
            self.default_public_key()?.to_bitcoin_address_typed(&net)?.as_external(),
            self.default_public_key()?.to_ethereum_address_typed()?.as_external(),
            self.solana_address()?.as_external(),
            self.monero_external_address(&NetworkEnvironment::Main)?.as_external()
        ])
    }

    fn metadata(&self) -> RgResult<WordsPassMetadata> {
        let default_calc = 20;
        let mut res = vec![];
        // Default spending keys
        let mut sequence = (0..default_calc).collect_vec().iter().map(|i| 50 + i).collect_vec();
        // Default peer ids
        sequence.extend((0..default_calc).collect_vec().iter().map(|i| 99 - i).collect_vec());
        for account in sequence {
            let path = format!("m/44'/0'/{}'/0/0", account);
            let xpub_path = format!("m/44'/0'/{}'", account);
            let pk = self.public_at(path.clone())?;
            res.push(WordsPassBtcMessageAccountMetadata {
                derivation_path: path.clone(),
                account,
                rdg_address: pk.address()?.render_string()?,
                rdg_btc_main_address: pk.to_bitcoin_address(&NetworkEnvironment::Main)?,
                rdg_btc_test_address: pk.to_bitcoin_address(&NetworkEnvironment::Test)?,
                xpub: self.xpub(xpub_path)?.to_string(),
                public_hex: pk.hex(),
            });
        }
        Ok(WordsPassMetadata {
            checksum: self.checksum()?,
            checksum_words: self.checksum_words()?,
            btc_84h_0h_0h_0_0_address: self.public_at("m/84'/0'/0'/0/0")?.to_bitcoin_address(&NetworkEnvironment::Main)?,
            btc_84h_0h_0h_0_0_xpub: self.xpub("m/84'/0'/0'")?.to_string(),
            eth_44h_60h_0h_0_0_address: self.public_at("m/44'/60'/0'/0/0")?.to_ethereum_address()?,
            eth_44h_60h_0h_0_0_xpub: self.xpub("m/44'/60'/0'")?.to_string(),
            rdg_44h_16180h_0h_0_0_address: self.public_at("m/44'/16180'/0'/0/0")?.address()?.render_string()?,
            rdg_44h_16180h_0h_0_0_xpub: self.xpub("m/44'/16180'/0'")?.to_string(),
            rdg_btc_message_account_metadata: res,
            executable_checksum: "".to_string(),
        })
    }

    fn default_rg_path(account: usize) -> String {
        default_node_internal_derivation_path(account as i64)
    }

    fn kp_rg_account(&self, account: usize) -> RgResult<KeyPair> {
        self.keypair_at(Self::default_rg_path(account))
    }

    fn default_kp(&self) -> RgResult<KeyPair> {
        self.kp_rg_account(0)
    }

    fn default_xpub(&self) -> RgResult<String> {
        let account_path = Self::default_rg_path(0).as_account_path().expect("works");
        self.xpub_str(account_path)
    }

    fn named_xpub(&self, key_name: impl Into<String>, skip_persist: bool, n: &NetworkEnvironment) -> RgResult<AccountKeySource> {
        self.default_xpub().map(|xpub| {
            let mut named = AccountKeySource::default();
            let key_into = key_name.into();
            named.name = format!("{}0", key_into);
            named.xpub = xpub;
            named.key_name_source = Some(key_into);
            named.request_type = Some(XPubLikeRequestType::Hot);
            named.skip_persist = Some(skip_persist);
            named.derivation_path = Self::default_rg_path(0);
            named.public_key = self.public_at(&named.derivation_path).ok();
            named.all_address = Some(named.public_key.as_ref().unwrap().to_all_addresses_for_network(n).unwrap());
            named
        })
    }

    fn default_pid_kp(&self) -> RgResult<KeyPair> {
        self.kp_rg_account(1)
    }

    fn checksum(&self) -> RgResult<String> {
        Ok(Hash::new_checksum(&self.seed()?.to_vec()))
    }

    fn checksum_words(&self) -> RgResult<String> {
        let mut s2 = self.clone();
        s2.passphrase = None;
        let s = s2.seed()?.to_vec();
        Ok(Hash::new_checksum(&s))
    }

    fn seed(&self) -> RgResult<[u8; 64]> {
        Ok(self.mnemonic()?.to_seed(self.passphrase.clone().unwrap_or("".to_string())))
    }

    fn hash_derive_words(&self, concat_nonce: impl Into<String>) -> RgResult<Self> {
        let mut vec = self.seed()?.to_vec();
        vec.extend(concat_nonce.into().as_bytes());
        let entropy = structs::Hash::digest(vec).raw_bytes()?;
        let m = Mnemonic::from_entropy(&*entropy).error_info("Failed to derive mnemonic from entropy")?;
        Ok(Self {
            words: m.to_string(),
            passphrase: None
        })
    }

    fn new_validated(words: impl Into<String>, passphrase: Option<String>) -> RgResult<Self> {
        let s = Self {
            words: words.into(),
            passphrase: passphrase.map(|p| p.into()),
        };
        s.validate()?;
        s.seed()?;
        Ok(s)
    }

    fn words(words: String) -> Self {
        Self {
            words,
            passphrase: None
        }
    }
    fn generate() -> RgResult<Self> {
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

    fn from_bytes(bytes: &[u8]) -> RgResult<Self> {
        let m = Mnemonic::from_entropy(bytes)
            .error_info("Failed to derive mnemonic from entropy")
            .with_detail("bytes", hex::encode(bytes))
            .with_detail("bytes_len", bytes.len().to_string())
            ?;
        Ok(Self {
            words: m.to_string(),
            passphrase: None
        })
    }

    fn from_str_hashed(str: impl Into<String>) -> Self {
        let b: Vec<u8> = structs::Hash::from_string_calculate(&str.into()).raw_bytes().expect("hash");
        Self::from_bytes(&b).unwrap()
    }

    fn mnemonic(&self) -> RgResult<Mnemonic> {
        Mnemonic::parse_in(
            Language::English,
            self.words.clone(),
        ).map_err(|e| error_info(format!("Failed to parse mnemonic: {}",
        e.to_string())))
    }

    fn validate(&self) -> RgResult<&Self> {
        self.mnemonic()?;
        Ok(self)
    }

    fn pair(&self) -> RgResult<(Mnemonic, Option<String>)> {
        Ok((self.mnemonic()?, self.passphrase.clone()))
    }

    fn extended_key(&self) -> RgResult<ExtendedKey> {
        self.pair()?.into_extended_key()
            .map_err(|e| error_info(format!("Failed to generate extended key: {}", e.to_string())))
    }

    fn xprv(&self) -> RgResult<ExtendedPrivKey> {
        self.extended_key()?.into_xprv(Network::Bitcoin)
            .safe_get_msg("Failed to generate xprv").cloned()
    }

    fn xpub(&self, path: impl Into<String>) -> RgResult<ExtendedPubKey> {
        let xprv = self.key_from_path_str(path.into())?;
        let xpub = ExtendedPubKey::from_priv(&Secp256k1::new(), &xprv);
        Ok(xpub)
    }

    fn derive_seed_at_path(&self, path: &str) -> RgResult<[u8; 32]> {
        let xprv = self.key_from_path_str(path)?;

        // Extract the 32-byte seed from the extended private key
        let seed = xprv.private_key.secret_bytes();

        Ok(seed)
    }

    fn xpub_str(&self, path: impl Into<String>) -> RgResult<String> {
        Ok(self.xpub(path)?.to_string())
    }

    fn key_from_path_str(&self, path: impl Into<String>) -> RgResult<ExtendedPrivKey> {
        let dp = DerivationPath::from_str(path.into().as_str())
            .error_info("Failed to parse derivation path")?;
        Ok(self.xprv()?.derive_priv(&Secp256k1::new(), &dp)
            .error_info("Failed to derive private key")?)
    }

    fn private_at(&self, path: impl Into<String>) -> RgResult<String> {
        let key = self.key_from_path_str(path)?;
        let pkhex = hex::encode(key.private_key.secret_bytes().to_vec());
        Ok(pkhex)
    }

    fn keypair_at(&self, path: impl Into<String>) -> RgResult<KeyPair> {
        KeyPair::from_private_hex(self.private_at(path)?)
    }

    fn public_at(&self, path: impl Into<String>) -> RgResult<structs::PublicKey> {
        let key = self.key_from_path_str(path.into())?;
        let vec = key.private_key.public_key(&Secp256k1::new()).serialize().to_vec();
        Ok(structs::PublicKey::from_bytes_direct_ecdsa(vec))
    }

    fn keypair_at_change(&self, change: impl Into<i64>) -> RgResult<KeyPair> {
        let key = self.keypair_at(redgold_keypair_change_path(change.into()))?;
        Ok(key)
    }

    fn default_peer_id(&self) -> RgResult<PeerId> {
        let pk = self.public_at(default_node_internal_derivation_path(1))?;
        let pid = PeerId::from_pk(pk);
        Ok(pid)
    }
    fn default_public_key(&self) -> RgResult<structs::PublicKey> {
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

    fn test_words() -> Self {
        WordsPass::new("abuse lock pledge crowd pair become ridge alone target viable black plate ripple sad tape victory blood river gloom air crash invite volcano release".to_string(), None)
    }
}

#[test]
fn generate_test() {
    let w = WordsPass::generate().expect("words");
    println!("{}", w.words.clone());
    assert_eq!(24, w.words.split(" ").collect_vec().len());
}

#[test]
fn generate_xpub() {
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
fn load_ci_kp() {
    if let Some(w) = std::env::var("REDGOLD_TEST_WORDS").ok() {
        let w = WordsPass::new(w, None);
        let path = "m/84'/0'/0'/0/0";
        let pk = w.public_at(path.to_string()).expect("private key");
        let w =
            SingleKeyBitcoinWallet::new_wallet(pk, NetworkEnvironment::Dev, true)
                .expect("w");
        let a = w.address().expect("a");
        println!("{a}");
    }
}