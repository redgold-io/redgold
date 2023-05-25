use redgold_schema::util::mnemonic_words::MnemonicWords;
use crate::api::RgHttpClient;
use crate::data::data_store::DataStore;

#[derive(Clone)]
struct Wallet {
    mnemonic_words: MnemonicWords,
    client: Option<RgHttpClient>,
    data_store: DataStore
}

impl Wallet {

    pub fn scan_initial(&self) {
        let mut words = self.mnemonic_words.clone();
        words.address();
    }
}