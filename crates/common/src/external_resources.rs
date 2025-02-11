use async_trait::async_trait;
use redgold_schema::structs::{Address, CurrencyAmount, ExternalTransactionId, MultisigRequest, MultisigResponse, NetworkEnvironment, PartySigningValidation, Proof, PublicKey, SupportedCurrency};
use redgold_schema::message::Response;
use redgold_schema::message::Request;
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use redgold_schema::{structs, RgResult};
use std::collections::HashMap;
use redgold_schema::keys::words_pass::WordsPass;

#[async_trait]
pub trait PeerBroadcast where Self: Sync + Clone + Send {
    async fn broadcast(&self, peers: &Vec<PublicKey>, request: Request) -> RgResult<Vec<RgResult<Response>>>;
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PartyCreationResult {
    pub address: Address,
    pub secret_json: Option<String>
}

#[async_trait]
pub trait ExternalNetworkResources where Self : Send + Clone + Sync {

    fn set_network(&mut self, network: &NetworkEnvironment);
    async fn get_all_tx_for_pk(&self, pk: &PublicKey, currency: SupportedCurrency, filter: Option<NetworkDataFilter>) -> RgResult<Vec<ExternalTimedTransaction>>;
    async fn get_all_tx_for_address(&self, address: &Address, currency: SupportedCurrency, filter: Option<NetworkDataFilter>)
                                    -> RgResult<Vec<ExternalTimedTransaction>>;
    async fn broadcast(&mut self, pk: &PublicKey, currency: SupportedCurrency, payload: EncodedTransactionPayload) -> RgResult<String>;
    async fn query_price(&self, time: i64, currency: SupportedCurrency) -> RgResult<f64>;
    async fn daily_historical_year(&self) -> RgResult<HashMap<SupportedCurrency, Vec<(i64, f64)>>>;
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

    async fn eth_tx_payload(&self, src: &Address, dst: &Address, amount: &CurrencyAmount, override_gas: Option<CurrencyAmount>) -> RgResult<(Vec<u8>, PartySigningValidation, String)>;

    async fn max_time_price_by(&self, currency: SupportedCurrency, max_time: i64) -> RgResult<Option<f64>>;

    async fn get_balance_no_cache(&self, network: &NetworkEnvironment, currency: &SupportedCurrency, pk: &PublicKey) -> RgResult<CurrencyAmount> where
        Self: Sync;

    async fn trezor_sign(&self, public: PublicKey, derivation_path: String, t: structs::Transaction) -> RgResult<structs::Transaction>;
    async fn participate_multisig_send(
        &self,
        mr: MultisigRequest,
        peer_pks: &Vec<PublicKey>,
        threshold: i64
    ) -> RgResult<MultisigResponse>;
    async fn execute_external_multisig_send<B: PeerBroadcast>(
        &self,
        destination_amounts: Vec<(Address, CurrencyAmount)>,
        party_address: &Address,
        peer_pks: &Vec<PublicKey>,
        broadcast: &B,
        threshold: i64
    ) -> RgResult<ExternalTransactionId>;

    async fn get_live_balance(&self, address: &Address) -> RgResult<CurrencyAmount>;

    async fn btc_pubkeys_to_multisig_address(&self, pubkeys: &Vec<PublicKey>, thresh: i64) -> RgResult<Address>;
    async fn create_multisig_party<B: PeerBroadcast>(
        &self,
        cur: &SupportedCurrency,
        all_pks: &Vec<PublicKey>,
        self_public_key: &PublicKey,
        self_private_key_hex: &String,
        network: &NetworkEnvironment,
        words_pass: WordsPass,
        threshold: i64,
        peer_broadcast: &B,
        peer_pks: &Vec<PublicKey>
    ) -> RgResult<Option<PartyCreationResult>>;


}

#[allow(dead_code)]
pub struct NetworkDataFilter {
    min_block: Option<u64>,
    min_time: Option<u64>
}

pub enum EncodedTransactionPayload {
    JsonPayload(String),
    BytesPayload(Vec<u8>)
}
