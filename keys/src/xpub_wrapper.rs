use std::str::FromStr;
use bdk::bitcoin::secp256k1::Secp256k1;
use bdk::bitcoin::util::bip32::{ChildNumber, ExtendedPubKey};
use bdk::miniscript::ToPublicKey;
use serde::{Deserialize, Serialize};
use redgold_schema::{ErrorInfoContext, RgResult, structs};

#[derive(Clone, Serialize, Deserialize)]
pub struct XpubWrapper{
    xpub: String
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
        Ok(structs::PublicKey::from_bytes(p.public_key.serialize().to_vec()))
    }
}