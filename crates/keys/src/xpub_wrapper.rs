use std::str::FromStr;
use bdk::bitcoin::secp256k1::Secp256k1;
use bdk::bitcoin::util::bip32::{ChildNumber, ExtendedPubKey};
use itertools::Itertools;

use serde::{Deserialize, Serialize};
use redgold_schema::{ErrorInfoContext, RgResult, SafeOption, structs};
use redgold_schema::proto_serde::ProtoSerde;
use crate::TestConstants;
use redgold_schema::keys::words_pass::WordsPass;
use crate::util::mnemonic_support::MnemonicSupport;

pub trait ValidateDerivationPath {
    fn valid_derivation_path(&self) -> bool;
    fn as_account_0_0(&self) -> Option<String>;
    fn as_account_path(&self) -> Option<String>;
}

impl ValidateDerivationPath for String {
    fn valid_derivation_path(&self) -> bool {
        WordsPass::words(TestConstants::new().words).public_at(self.clone()).is_ok()
    }
    fn as_account_0_0(&self) -> Option<String> {
        self.split("/").collect_vec().get(0..4)
            .map(|x| x.join("/").to_string() + "/0/0")
    }
    fn as_account_path(&self) -> Option<String> {
        self.split("/").collect_vec().get(0..4)
            .map(|x| x.join("/").to_string())
    }

    // fn valid_xpub_path(&self) -> bool {
    //     WordsPass::words(TestConstants::new().words).xprv(self.clone()).is_ok()
    // }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct XpubWrapper{
    pub xpub: String
}

impl XpubWrapper {
    pub fn new(xpub: String) -> Self {
        Self {
            xpub
        }
    }
    pub fn xpub(&self) -> RgResult<ExtendedPubKey> {
        ExtendedPubKey::from_str(&self.xpub).error_info("Failed to parse extended public key")
    }

    pub fn child_num(index: usize) -> RgResult<ChildNumber> {
        ChildNumber::from_normal_idx(index as u32).error_info(format!("Failed to parse child num {index}"))
    }
    pub fn public_at(&self, index: usize, change: usize) -> RgResult<structs::PublicKey> {
        let x = self.xpub()?;
        let x1 = vec![index, change].iter()
            .map(|i| Self::child_num(i.clone()))
            .collect::<RgResult<Vec<ChildNumber>>>()?;
        let p = x.derive_pub(&Secp256k1::new(), &x1).error_info("Failed to derive public key")?;
        Ok(structs::PublicKey::from_bytes_direct_ecdsa(p.public_key.serialize().to_vec()))
    }

    pub fn public_at_dp(&self, dp: &String) -> RgResult<structs::PublicKey> {
        let split = dp.split("/").collect::<Vec<&str>>();
        let index = split.get(4).ok_msg("Failed to find derivation path index")?
            .parse::<usize>().error_info("Failed to parse index")?;
        let change = split.get(5).ok_msg("Failed to find derivation path change")?
            .parse::<usize>().error_info("Failed to parse change")?;
        self.public_at(index, change)
    }
}


#[test]
pub fn test_xpub_wrapper() {
    let tc = TestConstants::new();
    let words = tc.words_pass;
    let test_dp = "m/44'/0'/50'/0/0".to_string();
    let account_path = test_dp.as_account_path().expect("works");
    let string = words.xpub_str(account_path).expect("works");
    let w = XpubWrapper::new(string);
    let public = words.public_at(test_dp.clone()).expect("works").hex();
    let public2 = w.public_at_dp(&test_dp).expect("works").hex();
    assert_eq!(public, public2);
}