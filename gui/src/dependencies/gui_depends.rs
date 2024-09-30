use serde::{Deserialize, Serialize};
use strum_macros::EnumString;
use redgold_schema::config_data::ConfigData;
use redgold_schema::RgResult;
use redgold_schema::structs::{AboutNodeResponse, AddressInfo, PublicKey, SubmitTransactionResponse, SupportedCurrency, Transaction};
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
    fn get_salt(&self) -> i64;
    async fn get_config(&self) -> ConfigData;
    async fn set_config(&self, config: ConfigData);
    async fn get_address_info(&self, pk: &PublicKey) -> RgResult<Option<AddressInfo>>;

    async fn submit_transaction(&self, tx: &Transaction) -> RgResult<SubmitTransactionResponse>;
    async fn about_node(&self) -> RgResult<AboutNodeResponse>;
    fn tx_builder(&self) -> TransactionBuilder;

    fn sign_transaction(&self, tx: &Transaction, sign_info: &TransactionSignInfo) -> RgResult<Transaction>;
}