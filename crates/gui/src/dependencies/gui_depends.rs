use crate::components::tx_progress::PreparedTransaction;
use crate::state::local_state::LocalStateUpdate;
use crate::tab::transact::states::DeviceListStatus;
use flume::Sender;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::config_data::ConfigData;
use redgold_schema::explorer::DetailedAddress;
use redgold_schema::keys::words_pass::WordsPass;
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::structs::{AboutNodeResponse, Address, AddressInfo, NetworkEnvironment, PublicKey, SubmitTransactionResponse, SupportedCurrency, Transaction};
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use redgold_schema::tx::tx_builder::TransactionBuilder;
use redgold_schema::RgResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct HardwareSigningInfo {
    pub path: String,
    pub message_to_sign: Option<String>,
    pub device_id: Option<String>
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct MnemonicWordsAndPassphrasePath {
    pub words: String,
    pub passphrase: Option<String>,
    pub path: Option<String>
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TransactionSignInfo {
    Mnemonic(MnemonicWordsAndPassphrasePath),
    PrivateKey(String),
    ColdOrAirgap(HardwareSigningInfo)
}

impl Default for TransactionSignInfo {
    fn default() -> Self {
        TransactionSignInfo::PrivateKey("".to_string())
    }
}

impl TransactionSignInfo {
    pub fn is_hot(&self) -> bool {
        match self {
            TransactionSignInfo::PrivateKey(_) => true,
            _ => false
        }
    }

    pub fn secret(&self) -> Option<String> {
        match self {
            TransactionSignInfo::PrivateKey(s) => Some(s.clone()),
            _ => None
        }
    }
}

pub trait GuiDepends {

    fn seed_checksum(m: WordsPass) -> RgResult<String>;
    fn hash_derive_words(m: WordsPass, derivation_path: impl Into<String>) -> RgResult<WordsPass>;
    fn public_at(m: WordsPass, derivation_path: impl Into<String>) -> RgResult<PublicKey>;
    fn private_at(m: WordsPass, derivation_path: impl Into<String>) -> RgResult<String>;
    fn checksum_words(m: WordsPass) -> RgResult<String>;
    fn private_hex_to_public_key(&self, hex: impl Into<String>) -> RgResult<PublicKey>;
    fn get_device_list_status(&self) -> DeviceListStatus;

    fn initial_queries_prices_parties_etc<E>(&self, sender: Sender<LocalStateUpdate>, ext: E) -> ()
    where E: ExternalNetworkResources + Send + 'static + Clone;
    fn network_changed(&self) -> flume::Receiver<NetworkEnvironment>;
    fn parse_address(&self, address: impl Into<String>) -> RgResult<Address>;
    fn set_network(&mut self, network: &NetworkEnvironment);
    fn get_network(&self) -> NetworkEnvironment;

    fn config_df_path_label(&self) -> Option<String>;
    fn get_salt(&self) -> i64;
    fn get_config(&self) -> ConfigData;
    fn set_config(&mut self, config: &ConfigData, allow_overwrite_all: bool);
    fn get_address_info(&self, pk: &PublicKey) -> impl std::future::Future<Output = RgResult<AddressInfo>> + Send;
    fn get_address_info_multi(&self, pk: Vec<&PublicKey>) -> impl std::future::Future<Output = Vec<RgResult<AddressInfo>> > + Send;

    fn submit_transaction(&self, tx: &Transaction) -> impl std::future::Future<Output = RgResult<SubmitTransactionResponse>> + Send;
    fn about_node(&self) -> impl std::future::Future<Output = RgResult<AboutNodeResponse>> + Send;
    fn tx_builder(&self) -> TransactionBuilder;

    fn sign_transaction(&self, tx: &Transaction, sign_info: &TransactionSignInfo) -> RgResult<Transaction>;
    fn sign_prepared_transaction(&mut self,
                                 tx: &PreparedTransaction,
                                 results: flume::Sender<RgResult<PreparedTransaction>>,
    ) -> RgResult<()>;
    fn broadcast_prepared_transaction(&mut self, tx: &PreparedTransaction, results: flume::Sender<RgResult<PreparedTransaction>>) -> RgResult<()>;
    fn spawn(&self, f: impl std::future::Future<Output = ()> + Send + 'static);
    fn spawn_interrupt(&self, f: impl std::future::Future<Output = ()> + Send + 'static, interrupt: flume::Receiver<()>);

    // this doesn't seem to work well with async functions
    fn spawn_blocking<T: Send+ 'static>(&self, f: impl std::future::Future<Output = RgResult<T>> + Send + 'static) -> RgResult<T>;

    fn validate_derivation_path(&self, derivation_path: impl Into<String>) -> bool;

    fn s3_checksum(&self) -> impl std::future::Future<Output = RgResult<String>> + Send;

    fn metrics(&self) -> impl std::future::Future<Output = RgResult<Vec<(String, String)>>> + Send;
    fn table_sizes(&self) -> impl std::future::Future<Output = RgResult<Vec<(String, i64)>>> + Send;
    fn party_data(&self) -> impl std::future::Future<Output = RgResult<HashMap<PublicKey, PartyInternalData>>> + Send;

    fn xpub_public(&self, xpub: String, path: String) -> RgResult<PublicKey>;

    async fn get_24hr_delta(&self, currency: SupportedCurrency) -> f64;

    fn get_detailed_address(&self, pk: &PublicKey) -> impl std::future::Future<Output = RgResult<Vec<DetailedAddress>>> + Send;

    fn get_external_tx(&mut self, pk: &PublicKey, currency: SupportedCurrency) -> impl std::future::Future<Output = RgResult<Vec<ExternalTimedTransaction>>> + Send;

    fn to_all_address(&self, pk: &PublicKey) -> Vec<Address>;

    fn form_eth_address(&self, pk: &PublicKey) -> RgResult<Address>;
    fn form_btc_address(&self, pk: &PublicKey) -> RgResult<Address>;

    fn backup_data_stores(&self) -> RgResult<()>;
    fn restore_data_stores(&self, filter: Option<Vec<i64>>) -> RgResult<()>;

}