use std::collections::HashMap;
use std::future::Future;
use flume::{Receiver, Sender};
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::config_data::ConfigData;
use redgold_schema::explorer::DetailedAddress;
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::RgResult;
use redgold_schema::structs::{AboutNodeResponse, Address, AddressInfo, NetworkEnvironment, PublicKey, SubmitTransactionResponse, SupportedCurrency, Transaction};
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use redgold_schema::tx::tx_builder::TransactionBuilder;
use redgold_schema::util::times::current_time_millis;
use crate::components::tx_progress::PreparedTransaction;
use crate::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};
use crate::state::local_state::LocalStateUpdate;

pub struct WasmGuiDepends {

}

impl GuiDepends for WasmGuiDepends {
    fn initial_queries_prices_parties_etc<E>(&self, sender: Sender<LocalStateUpdate>, ext: E) -> ()
    where
        E: ExternalNetworkResources + Send + 'static + Clone
    {
        todo!()
    }

    fn network_changed(&self) -> Receiver<NetworkEnvironment> {
        todo!()
    }

    fn parse_address(&self, address: impl Into<String>) -> RgResult<Address> {
        todo!()
    }

    fn set_network(&mut self, network: &NetworkEnvironment) {
        todo!()
    }


    fn get_network(&self) -> NetworkEnvironment {
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

    fn set_config(&mut self, config: &ConfigData) {
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

    fn sign_prepared_transaction(&mut self, tx: &PreparedTransaction, results: Sender<RgResult<PreparedTransaction>>) -> RgResult<()> {
        todo!()
    }
    fn broadcast_prepared_transaction(&mut self, tx: &PreparedTransaction, results: flume::Sender<RgResult<PreparedTransaction>>) -> RgResult<()> {
        todo!()
    }
    fn spawn(&self, f: impl Future<Output=()> + Send + 'static) {
        todo!()
    }

    fn spawn_interrupt(&self, f: impl Future<Output=()> + Send + 'static, interrupt: Receiver<()>) {
        todo!()
    }

    fn spawn_blocking<T: Send + 'static>(&self, f: impl Future<Output=RgResult<T>> + Send + 'static) -> RgResult<T> {
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

    async fn get_external_tx(&mut self, pk: &PublicKey, currency: SupportedCurrency) -> RgResult<Vec<ExternalTimedTransaction>> {
        todo!()
    }

    fn to_all_address(&self, pk: &PublicKey) -> Vec<Address> {
        todo!()
    }

    fn form_eth_address(&self, pk: &PublicKey) -> RgResult<Address> {
        todo!()
    }
    
    fn form_btc_address(&self, pk: &PublicKey) -> RgResult<Address> {
        todo!()
    }
}