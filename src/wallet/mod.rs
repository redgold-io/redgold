use crate::api::client::rest::RgHttpClient;
use redgold_data::data_store::DataStore;
use redgold_keys::util::mnemonic_support::WordsPass;

#[derive(Clone)]
struct Wallet {
    mnemonic_words: WordsPass,
    client: Option<RgHttpClient>,
    data_store: DataStore
}

impl Wallet {

    pub fn scan_initial(&self) {

    }
}