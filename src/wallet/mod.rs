use redgold_common::client::http::RgHttpClient;
use redgold_data::data_store::DataStore;
use redgold_schema::keys::words_pass::WordsPass;

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