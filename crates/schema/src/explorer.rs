use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, EnumString};
use std::collections::HashMap;
use crate::helpers::with_metadata_hashable::WithMetadataHashable;
use crate::party::central_price::CentralPricePair;
use crate::party::party_internal_data::PartyInternalData;
use crate::party::price_volume::PriceVolume;
use crate::{RgResult, SafeOption};
use crate::party::party_events::AddressEventExtendedType;
use crate::proto_serde::ProtoSerde;
use crate::structs::{ErrorInfo, SupportedCurrency, Transaction, TransactionType};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct HashResponse {
    pub hash: String,
    pub height: u64,
    pub timestamp: u64,
    pub transactions: Vec<String>,
}



#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct BriefTransaction {
    pub hash: String,
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub bytes: i64,
    pub timestamp: i64,
    pub first_amount: f64,
    pub is_test: bool,
    pub fee: i64,
    pub incoming: Option<bool>,
    pub currency: Option<String>,
    pub address_event_type: Option<AddressEventExtendedType>,
    pub num_signers: Option<i64>
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct PeerSignerDetailed {
    pub peer_id: String,
    pub nodes: Vec<NodeSignerDetailed>,
    pub trust: f64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct NodeSignerDetailed {
    pub signature: String,
    pub node_id: String,
    pub node_name: String,
    pub signed_pending_time: Option<i64>,
    pub signed_finalized_time: Option<i64>,
    pub observation_hash: String,
    pub observation_type: String,
    pub observation_timestamp: i64,
    pub validation_confidence_score: f64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct DetailedInput {
    pub transaction_hash: String,
    pub output_index: i64,
    pub address: String,
    pub input_amount: Option<f64>
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct UtxoChild {
    pub used_by_tx: String,
    pub used_by_tx_input_index: i32,
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct DetailedOutput {
    pub output_index: i32,
    pub address: String,
    pub available: bool,
    pub amount: f64,
    pub children: Vec<UtxoChild>,
    pub is_swap: bool,
    pub is_liquidity: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct PartyRelatedInfo {
    pub party_address: String,
    pub event: Option<DetailedPartyEvent>
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct DetailedTransaction {
    pub info: BriefTransaction,
    /// Normalized to mimic conventional confirmations, i.e. number of stacked observations
    /// Average weighted trust by peer observations
    pub confirmation_score: f64,
    pub acceptance_score: f64,
    pub message: String,
    pub num_pending_signers: i64,
    pub num_accepted_signers: i64,
    pub accepted: bool,
    pub signers: Vec<PeerSignerDetailed>,
    pub inputs: Vec<DetailedInput>,
    pub outputs: Vec<DetailedOutput>,
    pub rejection_reason: Option<ErrorInfo>,
    pub signable_hash: String,
    pub raw_transaction: Transaction,
    // pub first_amount: f64,
    pub remainder_amount: f64,
    pub party_related_info: Option<PartyRelatedInfo>,
    pub swap_info: Option<TransactionSwapInfo>,
    // pub fee_amount: i64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, EnumString, EnumIter)]
pub enum SwapStatus {
    Pending,
    Processing,
    Rejected,
    Complete,
    Refund
}

impl Default for SwapStatus {
    fn default() -> Self {
        SwapStatus::Pending
    }
}


#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct TransactionSwapInfo {
    pub party_address: String,
    pub swap_destination_address: String,
    pub input_currency: SupportedCurrency,
    pub output_currency: SupportedCurrency,
    pub swap_input_amount: String,
    pub swap_output_amount: Option<String>,
    pub swap_fee: Option<f64>,
    pub swap_input_amount_usd: String,
    pub swap_input_amount_usd_now: String,
    pub swap_output_amount_usd: Option<String>,
    pub swap_output_amount_usd_now: Option<String>,
    pub swap_status: SwapStatus,
    pub is_request: bool,
    pub is_fulfillment: bool,
    pub request_tx_hash: Option<String>,
    pub fulfillment_tx_hash: Option<String>,
    pub exchange_rate: Option<String>
}

#[derive(Serialize, Deserialize, EnumString)]
enum AddressEventType {
    Internal, External
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct DetailedPartyEvent {
    pub event_type: String,
    pub extended_type: String,
    pub other_address: String,
    pub network: String,
    pub amount: f64,
    pub tx_hash: String,
    pub incoming: String,
    pub time: i64,
    pub formatted_time: String,
    pub other_tx_hash: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AddressPoolInfo{
    pub public_key: String,
    // currency to address
    pub addresses: HashMap<String, String>,
    pub balances: HashMap<String, String>,
    pub bids: HashMap<String, Vec<PriceVolume>>,
    pub asks: HashMap<String, Vec<PriceVolume>>,
    pub bids_usd: HashMap<String, Vec<PriceVolume>>,
    pub asks_usd: HashMap<String, Vec<PriceVolume>>,
    pub central_prices: HashMap<String, CentralPricePair>,
    pub events: PartyInternalData,
    pub detailed_events: Vec<DetailedPartyEvent>,
    pub overall_staking_balances: Vec<(String, String)>,
    pub portfolio_staking_balances: Vec<(String, String)>,
    pub amm_staking_balances: Vec<(String, String)>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DetailedAddress {
    pub address: String,
    pub balance: f64,
    pub total_utxos: i64,
    pub recent_transactions: Vec<BriefTransaction>,
    pub utxos: Vec<BriefUtxoEntry>,
    pub incoming_transactions: Vec<BriefTransaction>,
    pub outgoing_transactions: Vec<BriefTransaction>,
    pub incoming_count: i64,
    pub outgoing_count: i64,
    pub total_count: i64,
    pub address_pool_info: Option<AddressPoolInfo>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct BriefUtxoEntry {
    pub transaction_hash: String,
    pub output_index: i64,
    pub amount: f64,
    pub time: i64
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct DetailedObservationMetadata {
    pub observed_hash: String,
    pub observed_hash_type: String,
    pub validation_type: String,
    pub state: String,
    pub validation_confidence: f64,
    pub time: i64,
    pub metadata_hash: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct DetailedObservation {
    pub merkle_root: String,
    pub observations: Vec<DetailedObservationMetadata>,
    pub public_key: String,
    pub signature: String,
    pub time: i64,
    pub hash: String,
    pub signable_hash: String,
    pub salt: i64,
    pub height: i64,
    pub parent_hash: String,
    pub peer_id: String
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct DetailedTrust {
    pub peer_id: String,
    pub trust: f64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct DetailedPeer {
    pub peer_id: String,
    pub nodes: Vec<DetailedPeerNode>,
    pub trust: Vec<DetailedTrust>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct DetailedPeerNode {
    pub external_address: String,
    pub public_key: String,
    pub node_type: String,
    pub executable_checksum: String,
    pub commit_hash: String,
    pub next_executable_checksum: String,
    pub next_upgrade_time: Option<i64>,
    pub utxo_distance: f64,
    pub port_offset: i64,
    pub node_name: String,
    pub peer_id: String,
    pub nat_restricted: bool,
    pub recent_observations: Vec<DetailedObservation>
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ExternalTxidInfo {
    pub external_explorer_link: String,
    pub party_address: String,
    pub event: DetailedPartyEvent,
    pub swap_info: Option<TransactionSwapInfo>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ExplorerHashSearchResponse {
    pub transaction: Option<DetailedTransaction>,
    pub address: Option<DetailedAddress>,
    pub observation: Option<DetailedObservation>,
    pub peer: Option<DetailedPeer>,
    pub peer_node: Option<DetailedPeerNode>,
    pub external_txid_info: Option<ExternalTxidInfo>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ExplorerFaucetResponse {
    pub transaction_hash: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RecentDashboardResponse {
    pub recent_transactions: Vec<BriefTransaction>,
    pub total_accepted_transactions: i64,
    pub size_transactions_gb: f64,
    pub total_accepted_utxos: i64,
    pub size_utxos_gb: f64,
    pub total_accepted_observations: i64,
    pub size_observations_gb: f64,
    pub total_distinct_utxo_addresses: i64,
    pub num_active_peers: i64,
    pub active_peers_abridged: Vec<DetailedPeer>,
    pub recent_observations: Vec<DetailedObservation>
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ExplorerPoolsResponse {
    pub pools: Vec<ExplorerPoolInfoResponse>
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct PoolMember {
    pub peer_id: String,
    pub public_key: String,
    pub share_fraction: f64,
    pub deposit_rating: f64,
    pub security_rating: f64,
    pub pool_stake_usd: Option<f64>,
    pub weighted_overall_stake_rating: Option<f64>,
    pub is_seed: bool
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct ExplorerPoolInfoResponse {
    pub public_key: String,
    pub owner: String,
    pub balance_btc: f64,
    pub balance_rdg: f64,
    pub balance_eth: f64,
    pub members: Vec<PoolMember>,
    pub threshold: f64,
}


pub fn check_address_event_type(tx: &Transaction) -> AddressEventExtendedType {

    if tx.is_swap() {
        AddressEventExtendedType::Swap
    } else if tx.is_swap_fulfillment() {
        return AddressEventExtendedType::SwapFulfillment;
    } else if tx.stake_deposit_request().is_some() {
        return AddressEventExtendedType::StakeDeposit;
    } else if tx.stake_withdrawal_request().is_some() {
        return AddressEventExtendedType::StakeWithdrawal;
    } else if tx.is_outgoing() {
        return AddressEventExtendedType::Send;
    } else {
        return AddressEventExtendedType::Receive;
    }
}

// TODO Make trait implicit
pub fn brief_transaction(tx: &Transaction, outgoing_from: Option<String>, num_signers: Option<i64>) -> RgResult<BriefTransaction> {
    let from_str = tx.first_input_address()
        .and_then(|a| a.render_string().ok())
        .unwrap_or("".to_string());
    Ok(BriefTransaction {
        hash: tx.hash_or().hex(),
        from: from_str.clone(),
        to: tx.first_output_address_non_input_or_fee().safe_get_msg("Missing output address")?.render_string()?,
        amount: tx.total_output_amount_float(),
        fee: tx.fee_amount(),
        bytes: tx.proto_serialize().len() as i64,
        timestamp: tx.struct_metadata.clone().and_then(|s| s.time).safe_get_msg("Missing tx timestamp")?.clone(),
        first_amount: tx.first_output_amount().safe_get_msg("Missing first output amount")?.clone(),
        is_test: tx.is_test(),
        incoming: outgoing_from.map(|i| i != from_str),
        currency: Some(SupportedCurrency::Redgold.to_display_string()),
        address_event_type: Some(check_address_event_type(tx)),
        num_signers,
    })
}