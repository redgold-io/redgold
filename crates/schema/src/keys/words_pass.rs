use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WordsPass {
    pub words: String,
    pub passphrase: Option<String>,
}

impl WordsPass {

    pub fn new(words: impl Into<String>, passphrase: Option<String>) -> Self {
        Self {
            words: words.into(),
            passphrase,
        }
    }

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