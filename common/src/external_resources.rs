use async_trait::async_trait;
use redgold_schema::{error_info, structs, RgResult};
use redgold_schema::structs::{Address, CurrencyAmount, ExternalTransactionId, NetworkEnvironment, PartySigningValidation, Proof, PublicKey, SupportedCurrency};
use redgold_schema::tx::external_tx::ExternalTimedTransaction;

#[async_trait]
pub trait ExternalNetworkResources {
    async fn get_all_tx_for_pk(&self, pk: &PublicKey, currency: SupportedCurrency, filter: Option<NetworkDataFilter>) -> RgResult<Vec<ExternalTimedTransaction>>;
    async fn broadcast(&mut self, pk: &PublicKey, currency: SupportedCurrency, payload: EncodedTransactionPayload) -> RgResult<String>;
    async fn query_price(&self, time: i64, currency: SupportedCurrency) -> RgResult<f64>;
    async fn send(&mut self, destination: &Address, currency_amount: &CurrencyAmount, broadcast: bool,
                  from: Option<PublicKey>, secret: Option<String>
    ) -> RgResult<(ExternalTransactionId, String)>;
    async fn self_balance(&self, currency: SupportedCurrency) -> RgResult<CurrencyAmount>;

    // EcdsaSighashType String
    async fn btc_payloads(
        &self, outputs: Vec<(String, u64)>, public_key: &PublicKey)
        -> RgResult<(Vec<(Vec<u8>, String)>, PartySigningValidation)>;
    async fn btc_add_signatures(
        &mut self, pk: &PublicKey, psbt: String,
        results: Vec<Proof>, hashes: Vec<(Vec<u8>, String)>) -> RgResult<EncodedTransactionPayload>;

    async fn eth_tx_payload(&self, src: &Address, dst: &Address, amount: &CurrencyAmount) -> RgResult<(Vec<u8>, PartySigningValidation, String)>;

    async fn max_time_price_by(&self, currency: SupportedCurrency, max_time: i64) -> RgResult<Option<f64>>;

    async fn get_balance_no_cache(&self, network: &NetworkEnvironment, currency: &SupportedCurrency, pk: &PublicKey) -> RgResult<CurrencyAmount>;

    async fn trezor_sign(&self, public: PublicKey, derivation_path: String, t: structs::Transaction) -> RgResult<structs::Transaction>;

}

pub struct NetworkDataFilter {
    min_block: Option<u64>,
    min_time: Option<u64>
}

pub enum EncodedTransactionPayload {
    JsonPayload(String),
    BytesPayload(Vec<u8>)
}
