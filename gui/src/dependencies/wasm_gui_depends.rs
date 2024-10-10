use std::collections::HashMap;
use std::future::Future;
use redgold_schema::config_data::ConfigData;
use redgold_schema::explorer::DetailedAddress;
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::RgResult;
use redgold_schema::structs::{AboutNodeResponse, AddressInfo, NetworkEnvironment, PublicKey, SubmitTransactionResponse, SupportedCurrency, Transaction};
use redgold_schema::tx::tx_builder::TransactionBuilder;
use redgold_schema::util::times::current_time_millis;
use crate::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};

pub struct WasmGuiDepends {

}

impl GuiDepends for WasmGuiDepends {
    fn set_network(&mut self, network: &NetworkEnvironment) {
        todo!()
    }

    fn config_df_path_label(&self) -> Option<String> {
        todo!()
    }

    fn get_salt(&self) -> i64 {
        let random = current_time_millis();
        random
    }

    fn get_config(&self) -> ConfigData {
        todo!()
    }

    fn set_config(&self, config: &ConfigData) {
        todo!()
    }

    async fn get_address_info(&self, pk: &PublicKey) -> RgResult<AddressInfo> {
        todo!()
    }

    async fn get_address_info_multi(&self, pk: Vec<&PublicKey>) -> Vec<RgResult<AddressInfo>> {
        todo!()
    }


    async fn submit_transaction(&self, tx: &Transaction) -> RgResult<SubmitTransactionResponse> {
        todo!()
    }

    async fn about_node(&self) -> RgResult<AboutNodeResponse> {
        todo!()
    }

    fn tx_builder(&self) -> TransactionBuilder {
        todo!()
    }

    fn sign_transaction(&self, tx: &Transaction, sign_info: &TransactionSignInfo) -> RgResult<Transaction> {
        todo!()
    }

    fn spawn(&self, f: impl Future<Output=()> + Send + 'static) {
        todo!()
    }

    fn validate_derivation_path(&self, derivation_path: impl Into<String>) -> bool {
        todo!()
    }

    async fn s3_checksum(&self) -> RgResult<String> {
        todo!()
    }

    async fn metrics(&self) -> RgResult<Vec<(String, String)>> {
        todo!()
    }

    async fn table_sizes(&self) -> RgResult<Vec<(String, i64)>> {
        todo!()
    }

    async fn party_data(&self) -> RgResult<HashMap<PublicKey, PartyInternalData>> {
        todo!()
    }

    fn xpub_public(&self, xpub: String, path: String) -> RgResult<PublicKey> {
        todo!()
    }

    async fn get_24hr_delta(&self, currency: SupportedCurrency) -> f64 {
        todo!()
    }

    async fn get_detailed_address(&self, pk: &PublicKey) -> RgResult<Vec<DetailedAddress>> {
        todo!()
    }
}