use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use strum_macros::EnumString;
use redgold_schema::config_data::ConfigData;
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::RgResult;
use redgold_schema::structs::{AboutNodeResponse, AddressInfo, NetworkEnvironment, PublicKey, SubmitTransactionResponse, SupportedCurrency, Transaction};
use redgold_schema::tx::tx_builder::TransactionBuilder;


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct HardwareSigningInfo {
    pub path: String,
    pub message_to_sign: Option<String>,
    pub device_id: Option<String>
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TransactionSignInfo {
    PrivateKey(String),
    ColdHardwareWallet(HardwareSigningInfo),
    Qr(QrMessage)
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, EnumString)]
pub enum QrMessage {
    SignTransaction(Transaction),
    SignTransactions(Vec<Transaction>),
}

pub trait GuiDepends {

    fn set_network(&mut self, network: &NetworkEnvironment);

    fn config_df_path_label(&self) -> Option<String>;
    fn get_salt(&self) -> i64;
    fn get_config(&self) -> ConfigData;
    fn set_config(&self, config: &ConfigData);
    fn get_address_info(&self, pk: &PublicKey) -> impl std::future::Future<Output = RgResult<AddressInfo>> + Send;
    fn get_address_info_multi(&self, pk: Vec<&PublicKey>) -> impl std::future::Future<Output = Vec<RgResult<AddressInfo>> > + Send;

    fn submit_transaction(&self, tx: &Transaction) -> impl std::future::Future<Output = RgResult<SubmitTransactionResponse>> + Send;
    fn about_node(&self) -> impl std::future::Future<Output = RgResult<AboutNodeResponse>> + Send;
    fn tx_builder(&self) -> TransactionBuilder;

    fn sign_transaction(&self, tx: &Transaction, sign_info: &TransactionSignInfo) -> RgResult<Transaction>;

    fn spawn(&self, f: impl std::future::Future<Output = ()> + Send + 'static);

    fn validate_derivation_path(&self, derivation_path: impl Into<String>) -> bool;

    fn s3_checksum(&self) -> impl std::future::Future<Output = RgResult<String>> + Send;

    fn metrics(&self) -> impl std::future::Future<Output = RgResult<Vec<(String, String)>>> + Send;
    fn table_sizes(&self) -> impl std::future::Future<Output = RgResult<Vec<(String, i64)>>> + Send;
    fn party_data(&self) -> impl std::future::Future<Output = RgResult<HashMap<PublicKey, PartyInternalData>>> + Send;

    fn xpub_public(&self, xpub: String, path: String) -> RgResult<PublicKey>;
}