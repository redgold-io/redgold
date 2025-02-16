use crate::components::tx_progress::PreparedTransaction;
use crate::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};
use crate::state::local_state::LocalStateUpdate;
use crate::tab::transact::states::DeviceListStatus;
use flume::{Receiver, Sender};
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::config_data::ConfigData;
use redgold_schema::explorer::DetailedAddress;
use redgold_schema::keys::words_pass::{WordsPass, WordsPassMetadata};
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::structs::{AboutNodeResponse, Address, AddressInfo, NetworkEnvironment, PublicKey, SubmitTransactionResponse, SupportedCurrency, Transaction};
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use redgold_schema::tx::tx_builder::TransactionBuilder;
use redgold_schema::util::times::current_time_millis;
use redgold_schema::RgResult;
use std::collections::HashMap;
use std::future::Future;

pub struct WasmGuiDepends {

}

impl GuiDepends for WasmGuiDepends {
    fn mnemonic_builder_from_str_rounds(str: &String, rounds: usize) -> WordsPass {
        todo!()
    }

    fn mnemonic_to_seed(w: WordsPass) -> Vec<u8> {
        todo!()
    }

    fn words_pass_metadata(w: WordsPass) -> WordsPassMetadata {
        todo!()
    }

    fn generate_random_mnemonic() -> WordsPass {
        todo!()
    }

    fn get_cold_xpub(dp: String) -> RgResult<String> {
        todo!()
    }

    fn seed_checksum(m: WordsPass) -> RgResult<String> {
        todo!()
    }

    fn hash_derive_words(m: WordsPass, derivation_path: impl Into<String>) -> RgResult<WordsPass> {
        todo!()
    }

    fn public_at(m: WordsPass, derivation_path: impl Into<String>) -> RgResult<PublicKey> {
        todo!()
    }

    fn private_at(m: WordsPass, derivation_path: impl Into<String>) -> RgResult<String> {
        todo!()
    }

    fn checksum_words(m: WordsPass) -> RgResult<String> {
        todo!()
    }

    fn private_hex_to_public_key(&self, hex: impl Into<String>) -> RgResult<PublicKey> {
        todo!()
    }

    fn get_device_list_status(&self) -> DeviceListStatus {
        DeviceListStatus::default()
    }

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

    fn set_config(&mut self, config: &ConfigData, over_all: bool) {
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

    fn backup_data_stores(&self) -> RgResult<()> {
        Ok(())
    }

    fn restore_data_stores(&self, filter: Option<Vec<i64>>) -> RgResult<()> {
        Ok(())
    }

    fn validate_mnemonic(w: WordsPass) -> RgResult<()> {
        todo!()
    }

    fn argon2d_hash(salt: Vec<u8>, nonce: Vec<u8>, m_cost: u32, t_cost: u32, p_cost: u32) -> RgResult<Vec<u8>> {
        todo!()
    }

    fn words_pass_from_bytes(bytes: &[u8]) -> RgResult<WordsPass> {
        todo!()
    }

    fn as_account_path(path: impl Into<String>) -> Option<String> {
        todo!()
    }

    fn get_xpub_string_path(w: WordsPass, path: impl Into<String>) -> RgResult<String> {
        todo!()
    }
}