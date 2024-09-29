use redgold_schema::config_data::ConfigData;
use redgold_schema::RgResult;
use redgold_schema::structs::{AboutNodeResponse, AddressInfo, PublicKey, SubmitTransactionResponse, SupportedCurrency, Transaction};
use redgold_schema::tx::tx_builder::TransactionBuilder;

pub trait GuiDepends {
    fn get_salt(&self) -> i64;
    async fn get_config(&self) -> ConfigData;
    async fn set_config(&self, config: ConfigData);
    async fn get_address_info(&self, pk: &PublicKey) -> RgResult<Option<AddressInfo>>;

    async fn submit_transaction(&self, tx: &Transaction) -> RgResult<SubmitTransactionResponse>;
    async fn about_node(&self) -> RgResult<AboutNodeResponse>;
    fn tx_builder(&self) -> TransactionBuilder;
}