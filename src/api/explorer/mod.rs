pub mod server;
pub mod debug_test;

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::convert::identity;
use std::hash::Hash;
use std::net::SocketAddr;
use std::time::Duration;
use eframe::egui::accesskit::Role::Math;
use futures::TryFutureExt;
use itertools::Itertools;
use log::info;
use rocket::form::FromForm;
use rocket::yansi::Paint;
use redgold_schema::{error_info, RgResult, SafeOption};
use crate::api::hash_query::hash_query;
use crate::core::relay::Relay;
use serde::{Deserialize, Serialize};
use redgold_data::peer::PeerTrustQueryResult;
use redgold_schema::structs::{Address, AddressInfo, CurrencyAmount, ErrorInfo, FaucetRequest, FaucetResponse, HashType, NetworkEnvironment, NodeType, Observation, ObservationMetadata, PartyInfo, PeerId, PeerIdInfo, PeerNodeInfo, PublicKey, QueryTransactionResponse, Request, State, SubmitTransactionResponse, SupportedCurrency, Transaction, TransactionInfo, TrustRatingLabel, UtxoEntry, ValidationType};
use strum_macros::EnumString;
use tokio::time::Instant;
use tracing::trace;
use warp::get;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_schema::transaction::{rounded_balance, rounded_balance_i64};
use crate::api::public_api::{Pagination, TokenParam};
// use crate::multiparty_gg20::watcher::{DepositWatcher, DepositWatcherConfig};
use crate::util;
use redgold_keys::address_external::ToBitcoinAddress;
use redgold_keys::address_support::AddressSupport;
use redgold_keys::proof_support::PublicKeySupport;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use crate::api::faucet::faucet_request;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::proto_serde::ProtoSerde;
use crate::util::current_time_millis_i64;
use redgold_schema::structs::PartyInfoAbridged;
use redgold_schema::util::times::ToTimeString;
use crate::integrations::external_network_resources::ExternalNetworkResourcesImpl;
use redgold_schema::conf::node_config::NodeConfig;
use crate::node_config::ApiNodeConfig;
use crate::party::address_event::AddressEvent;
use crate::party::central_price::CentralPricePair;
use crate::party::data_enrichment::PartyInternalData;
use crate::party::price_volume::PriceVolume;
// use crate::party::bid_ask::BidAsk;

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct HashResponse {
    pub hash: String,
    pub height: u64,
    pub timestamp: u64,
    pub transactions: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct BriefTransaction {
    pub hash: String,
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub bytes: i64,
    pub timestamp: i64,
    pub first_amount: f64,
    pub is_test: bool,
    pub fee: i64
}



#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct PeerSignerDetailed {
    pub peer_id: String,
    pub nodes: Vec<NodeSignerDetailed>,
    pub trust: f64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
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


#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct DetailedInput {
    pub transaction_hash: String,
    pub output_index: i64,
    pub address: String,
    pub input_amount: Option<f64>
}


#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct UtxoChild {
    pub used_by_tx: String,
    pub used_by_tx_input_index: i32,
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct DetailedOutput {
    pub output_index: i32,
    pub address: String,
    pub available: bool,
    pub amount: f64,
    pub children: Vec<UtxoChild>,
    pub is_swap: bool,
    pub is_liquidity: bool,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct PartyRelatedInfo {
    party_address: String,
    event: Option<DetailedPartyEvent>
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
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
    pub party_related_info: Option<PartyRelatedInfo>
    // pub fee_amount: i64,
}




#[derive(Serialize, Deserialize, EnumString)]
enum AddressEventType {
    Internal, External
}


#[derive(Serialize, Deserialize, EnumString)]
enum AddressEventExtendedType {
    StakeDeposit, StakeWithdrawal, Swap
}


#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct DetailedPartyEvent {
    event_type: String,
    extended_type: String,
    other_address: String,
    network: String,
    amount: f64,
    tx_hash: String,
    incoming: String,
    time: i64,
    formatted_time: String,
    pub other_tx_hash: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AddressPoolInfo{
    public_key: String,
    // currency to address
    addresses: HashMap<String, String>,
    balances: HashMap<String, String>,
    bids: HashMap<String, Vec<PriceVolume>>,
    asks: HashMap<String, Vec<PriceVolume>>,
    bids_usd: HashMap<String, Vec<PriceVolume>>,
    asks_usd: HashMap<String, Vec<PriceVolume>>,
    central_prices: HashMap<String, CentralPricePair>,
    events: PartyInternalData,
    detailed_events: Vec<DetailedPartyEvent>
}

#[derive(Serialize, Deserialize, Clone)]
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


#[derive(Serialize, Deserialize, Clone, PartialEq)]
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
    pub event: DetailedPartyEvent
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

pub fn convert_utxo(u: &UtxoEntry) -> RgResult<BriefUtxoEntry> {
    let id = u.utxo_id.safe_get_msg("missing utxo id")?;
    let amt = u.opt_amount().map(|a| a.to_fractional()).unwrap_or(0.0);
    Ok(BriefUtxoEntry {
        transaction_hash: id.transaction_hash.safe_get_msg("Missing transaction hash")?.hex(),
        output_index: id.output_index.clone(),
        amount: amt,
        time: u.time.clone(),
    })
}

pub async fn get_address_pool_info(r: Relay) -> RgResult<Option<AddressPoolInfo>> {

    let data = r.external_network_shared_data.clone_read().await;
    let pid = data
        .iter()
        .filter(|(k, v)| v.party_info.self_initiated())
        .next()
        .map(|x| x.1);
    if let Some(d) = pid {
        let pk = d.party_info.party_key.safe_get_msg("Missing party key")?;
        let public_key =
            pk.hex();
        if let Some(pe) = d.party_events.as_ref() {
            let balances = pe.balance_map.iter().map(|(k, v)| {
                (format!("{:?}", k), v.to_fractional().to_string())
            }).collect::<HashMap<String, String>>();
            let addresses = pk.to_all_addresses_for_network_by_currency(&r.node_config.network)?
                .iter().flat_map(|(c,a)| a.render_string().ok()
                .map(|aa| (format!("{:?}", c), aa))).collect::<HashMap<String, String>>();
            let central_prices = pe.central_prices.iter().map(|(k,v)| {
                (format!("{:?}", k), v.clone())
            }).collect::<HashMap<String, CentralPricePair>>();
            let bids = pe.central_prices.iter().map(|(k,v)| {
                (format!("{:?}", k), v.bids().clone())
            }).collect::<HashMap<String, Vec<PriceVolume>>>();
            let asks = pe.central_prices.iter().map(|(k,v)| {
                (format!("{:?}", k), v.asks().clone())
            }).collect::<HashMap<String, Vec<PriceVolume>>>();
            let bids_usd = pe.central_prices.iter().map(|(k,v)| {
                (format!("{:?}", k), v.bids_usd().clone())
            }).collect::<HashMap<String, Vec<PriceVolume>>>();
            let asks_usd = pe.central_prices.iter().map(|(k,v)| {
                (format!("{:?}", k), v.asks_usd().clone())
            }).collect::<HashMap<String, Vec<PriceVolume>>>();

            let mut d = d.clone();
            let events = d.clear_sensitive().clone();

            return Ok(Some(AddressPoolInfo {
                public_key,
                addresses,
                balances,
                bids,
                asks,
                bids_usd,
                asks_usd,
                central_prices,
                events: events.clone(),
                detailed_events: convert_events(&events, &r.node_config)?,
            }))
        };


    }
    Ok(None)
}

pub fn convert_events(p0: &PartyInternalData, nc: &NodeConfig) -> RgResult<Vec<DetailedPartyEvent>> {
    let mut res = vec![];
    for x in p0.address_events.iter() {
        let mut de = DetailedPartyEvent {
            event_type: "".to_string(),
            extended_type: "".to_string(),
            network: "".to_string(),
            amount: 0.0,
            tx_hash: "".to_string(),
            other_address: "".to_string(),
            incoming: "".to_string(),
            time: 0,
            formatted_time: "".to_string(),
            other_tx_hash: "".to_string(),
        };
        if let Some(time) = x.time(&nc.seeds_now_pk()) {
            de.time = time;
            de.formatted_time = time.to_time_string_shorter();
        }
        let x2 = x.clone();
        match x {
            AddressEvent::External(ett) => {
                de.event_type = "External".to_string();
                if let Some(ps) = p0.party_events.as_ref() {
                    de.extended_type = {
                        let swap = ps.fulfillment_history.iter().filter(|h| h.1 == x2).next();
                        let stake_fulfill = ps.external_staking_events.iter().filter(|e| e.event == x2).next();
                        let swap_fulfillment = ps.fulfillment_history.iter().filter(|h| h.2 == x2).next();
                        if let Some(stake_fulfill) = stake_fulfill {
                            de.other_tx_hash = stake_fulfill.pending_event.tx.hash_or().hex();
                            "StakeDepositFulfillment"
                        } else if let Some(swap) = swap {
                            de.other_tx_hash = swap.2.identifier();
                            "Swap"
                        } else if let Some(swap_fulfillment) = swap_fulfillment {
                            de.other_tx_hash = swap_fulfillment.1.identifier();
                            "SwapFulfillment"
                        } else {
                            "Pending/Unknown"
                        }
                    }.to_string();
                };
                de.incoming = ett.incoming.to_string();
                de.network = format!("{:?}", ett.currency);
                de.amount = ett.currency_amount().to_fractional();
                de.tx_hash = ett.tx_id.clone();
                de.other_address = ett.other_address.clone();
            }
            AddressEvent::Internal(i) => {
                de.event_type = "Internal".to_string();
                de.extended_type = {
                    // Okay technically this first type should be a Vec<String> but let's ignore that for now.
                    if let Some(sf) = i.tx.swap_fulfillment() {
                        de.other_tx_hash = sf.external_transaction_id.as_ref().map(|t| t.identifier.clone()).unwrap_or("".to_string());
                        "SwapFulfillment"
                    } else if i.tx.is_swap() {
                        "Swap"
                    } else if i.tx.is_stake() {
                        "Stake"
                    } else {
                        "Unknown"
                    }
                }.to_string();
                let party_key = p0.party_info.party_key.clone().expect("k");
                let party_address = party_key.address().expect("address");
                let inc = !i.tx.inputs_match_pk_address(&party_address);
                de.incoming = format!("{}", inc);
                de.amount = {
                    if inc {
                        i.tx.output_rdg_amount_of_pk(&party_key).map(|a| a.to_fractional())
                            .unwrap_or(0f64)
                    } else {
                        i.tx.output_rdg_amount_of_exclude_pk(&party_key).map(|a| a.to_fractional())
                            .unwrap_or(0f64)
                    }
                };
                de.other_address = {
                    if inc {
                        i.tx.first_input_address()
                    } else {
                        i.tx.first_output_address_non_input_or_fee()
                    }
                }.and_then(|a| a.render_string().ok()).unwrap_or("".to_string());
                de.tx_hash = i.tx.hash_or().hex();
                de.network = "Redgold".to_string();
            }
        };
        res.push(de);
    };
    Ok(res)
}

pub async fn handle_address_info(ai: &AddressInfo, r: &Relay, limit: Option<i64>, offset: Option<i64>) -> RgResult<DetailedAddress> {

    let a = ai.address.safe_get_msg("Missing address")?.clone();
    let recent: Vec<Transaction> = ai.recent_transactions.clone();
    let incoming_transactions = r.ds.transaction_store.get_filter_tx_for_address(
        &a, limit.unwrap_or(10), offset.unwrap_or(0), true
    ).await?.iter().map(|u| brief_transaction(&u)).collect::<RgResult<Vec<BriefTransaction>>>()?;
    let outgoing_transactions = r.ds.transaction_store.get_filter_tx_for_address(
        &a, limit.unwrap_or(10), offset.unwrap_or(0), false
    ).await?.iter().map(|u| brief_transaction(&u)).collect::<RgResult<Vec<BriefTransaction>>>()?;

    let incoming_count = r.ds.transaction_store.get_count_filter_tx_for_address(&a, true).await?;
    let outgoing_count = r.ds.transaction_store.get_count_filter_tx_for_address(&a, false).await?;
    let total_count = incoming_count.clone() + outgoing_count.clone();

    let address_str = a.render_string()?;
    let address_pool_info = get_address_pool_info(r.clone()).await?
        .filter(|p| p.addresses.values().collect_vec().contains(&&address_str));

    let detailed = DetailedAddress {
        address: address_str,
        balance: rounded_balance_i64(ai.balance.clone()),
        total_utxos: ai.utxo_entries.len() as i64,
        recent_transactions: recent.iter().map(|u| brief_transaction(&u)).collect::<RgResult<Vec<BriefTransaction>>>()?,
        utxos: ai.utxo_entries.iter().map(|u| convert_utxo(u)).collect::<RgResult<Vec<BriefUtxoEntry>>>()?,
        incoming_transactions,
        outgoing_transactions,
        incoming_count,
        outgoing_count,
        total_count,
        address_pool_info,
    };
    Ok(detailed)
}


pub fn convert_observation_metadata(om: &ObservationMetadata) -> RgResult<DetailedObservationMetadata> {
    Ok(DetailedObservationMetadata{
        observed_hash: om.observed_hash.safe_get()?.clone().hex(),
        observed_hash_type:
        format!("{:?}", HashType::from_i32(om.observed_hash.safe_get()?.clone().hash_type).safe_get()?.clone()),
        validation_type:
        format!("{:?}", ValidationType::from_i32(om.observation_type.clone()).safe_get()?.clone()),
        state:
        format!("{:?}", State::from_i32(om.state.clone()).safe_get()?.clone()),
        validation_confidence: om.validation_confidence.as_ref()
            .map(|l| l.label() * 10.0)
            .unwrap_or(10.0),
        time: om.struct_metadata.safe_get()?.time.safe_get()?.clone(),
        metadata_hash: om.struct_metadata.safe_get()?.hash.safe_get()?.hex(),
    })
}

pub async fn handle_observation(otx: &Transaction, _r: &Relay) -> RgResult<DetailedObservation> {

    let o = otx.observation()?;

    let pbs_pk = otx.observation_public_key()?;
    Ok(DetailedObservation {
        merkle_root: o.merkle_root.safe_get()?.hex(),
        observations: o.observations.iter()
            .map(|om| convert_observation_metadata(om))
            .collect::<RgResult<Vec<DetailedObservationMetadata>>>()?,
        public_key: pbs_pk.hex(),
        signature: otx.observation_proof()?.signature_hex()?,
        time: otx.time()?.clone(),
        hash: otx.hash_or().hex(),
        signable_hash: otx.signable_hash().hex(),
        salt: otx.salt()?,
        height: otx.height()?,
        parent_hash: o.parent_id.as_ref().and_then(|h| h.transaction_hash.as_ref().map(|h| h.hex())).unwrap_or("".to_string()),
        peer_id: _r.peer_id_for_node_pk(pbs_pk).await.ok().flatten()
            .map(|p| p.hex()).unwrap_or("".to_string()),
    })
}

pub fn convert_trust(trust: &TrustRatingLabel) -> RgResult<DetailedTrust> {
    let pid: Option<PeerId> = trust.peer_id.clone();
    let h = pid.and_then(|p| p.peer_id).map(|h| h.hex()).unwrap_or("".to_string());
    Ok(DetailedTrust{
        peer_id: h,
        trust: trust.trust_data.get(0).safe_get()?.label(),
    })
}

pub async fn handle_peer(p: &PeerIdInfo, r: &Relay, skip_recent_observations: bool) -> RgResult<DetailedPeer> {
    let pd = p.latest_peer_transaction.safe_get_msg("Missing latest peer transaction in handle peer")?
        .peer_data()?;
    let mut nodes = vec![];
    for pni in &p.peer_node_info {
        let node = handle_peer_node(pni, &r, skip_recent_observations).await.log_error();
        if let Ok(node) = node {
            nodes.push(node);
        }
    }
    Ok(DetailedPeer {
        peer_id: pd.peer_id.safe_get_msg("Missing peer id")?.peer_id
            .safe_get_msg("Missing peer id public key info")?.hex(),
        nodes,
        trust: pd.labels.iter().map(|l| convert_trust(l))
            .collect::<RgResult<Vec<DetailedTrust>>>()?
    })
}

pub async fn handle_peer_node(p: &PeerNodeInfo, _r: &Relay, skip_recent_observations: bool) -> RgResult<DetailedPeerNode> {
    let nmd = p.latest_node_transaction.safe_get_msg("Missing latest node transaction")?.node_metadata()?;
    let vi = nmd.version_info.clone().safe_get_msg("Missing version info")?.clone();
    let pk = nmd.public_key.safe_get_msg("Missing public key")?;
    let mut obs = vec![];

    if !skip_recent_observations {
        for o in _r.ds.observation.get_pk_observations(pk, 10).await? {
            let oo = handle_observation(&o, _r).await?;
            obs.push(oo);
        }
    }

    Ok(DetailedPeerNode{
        external_address: nmd.external_address()?,
        public_key: pk.hex(),
        node_type:  format!("{:?}", NodeType::from_i32(nmd.node_type.unwrap_or(0)).unwrap_or(NodeType::Static)),
        executable_checksum: vi.executable_checksum.clone(),
        commit_hash: vi.commit_hash.unwrap_or("".to_string()),
        next_executable_checksum: vi.next_executable_checksum.clone().unwrap_or("".to_string()),
        next_upgrade_time: vi.next_upgrade_time.clone(),
        utxo_distance: nmd.partition_info.as_ref()
            .and_then(|p| p.utxo)
            .map(|d| (d as f64) / 1000 as f64) // TODO: use a function for this
            .unwrap_or(1.0),
        port_offset: nmd.port_or(_r.node_config.network) as i64,
        node_name: nmd.node_name.unwrap_or("".to_string()),
        peer_id: nmd.peer_id.as_ref()
            .map(|p| p.hex()).unwrap_or("".to_string()),
        nat_restricted: nmd.transport_info.as_ref().and_then(|t| t.nat_restricted).unwrap_or(false),
        recent_observations: obs,
    })
}

pub async fn handle_explorer_faucet(hash_input: String, r: Relay, token: TokenParam, origin: Option<String>) -> RgResult<ExplorerFaucetResponse> {
    let res = async { hash_input.parse_address() }.and_then(|a| {
        let mut req = Request::default();
        req.origin = origin;
        let mut fr = FaucetRequest::default();
        fr.address = Some(a);
        fr.token = token.token;
        req.faucet_request = Some(fr);
        r.receive_request_send_internal(req, None)
    }).await?;
    res.as_error_info()?;
    let fr: &FaucetResponse = res.faucet_response.safe_get_msg("Missing faucet response")?;
    let submit = fr.submit_transaction_response.safe_get_msg("Missing submit transaction response")?;
    let h = submit.transaction_hash.safe_get_msg("Missing transaction hash")?.hex();
    Ok(ExplorerFaucetResponse{
        transaction_hash: Some(h),
    })
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


async fn render_pool_member(relay: &Relay, member: &PublicKey, party_len: usize) -> RgResult<PoolMember> {
    Ok(PoolMember {
        peer_id: relay.ds.peer_store.peer_id_for_node_pk(member).await?.map(|p| p.hex()).unwrap_or("".to_string()),
        public_key: member.hex(),
        share_fraction: 1f64/(party_len as f64),
        deposit_rating: 10.0,
        security_rating: 10.0,
        pool_stake_usd: None,
        weighted_overall_stake_rating: None,
        is_seed: relay.is_seed(member).await,
    })
}


async fn convert_party_info(relay: &Relay, pi: &PartyInfoAbridged) -> RgResult<ExplorerPoolInfoResponse> {
    if let Some(pid) = pi.party_id.as_ref() {
        if let Some(pk) = pid.public_key.as_ref() {
            if let Some(owner) = pid.owner.as_ref() {
                if let Some(thresh) = pi.threshold.as_ref() {
                    let thresh = thresh.to_float();
                    let num_members = pi.members.len();
                    let mut members = vec![];
                    for member in &pi.members {
                        if let Some(member_pk) = &member.public_key {
                            if let Some(weight) = &member.weight {
                                let member = render_pool_member(&relay, &member_pk, num_members).await?;
                                members.push(member);
                            }
                        }
                    }
                    let balance_btc = pi.balances.iter().filter(|b| b.currency == Some(SupportedCurrency::Bitcoin as i32)).next()
                        .map(|b| b.amount).unwrap_or(0);

                    let balance_rdg = pi.balances.iter().filter(|b| b.currency == Some(SupportedCurrency::Redgold as i32)).next()
                        .map(|b| b.amount).unwrap_or(0);

                    return Ok(ExplorerPoolInfoResponse {
                        public_key: pk.hex(),
                        owner: owner.hex(),
                        balance_btc: (balance_btc as f64) / 1e8,
                        balance_rdg: (balance_rdg as f64) / 1e8,
                        balance_eth: 0.0,
                        members,
                        threshold: thresh
                    })
                }
            }
        }
    }
    Err(error_info("Missing party info"))

}
async fn handle_explorer_pool(relay: Relay) -> RgResult<ExplorerPoolsResponse> {
    let pools = vec![];
    // if let Some(dw) = DepositWatcher::get_deposit_config(&relay.ds).await? {
    //     let opt = dw.deposit_allocations.get(0);
    //     if let Some(dk) = opt {
    //        if let Ok(pi) = dk.party_info() {
    //            let res = convert_party_info(&relay, &pi).await?;
    //             pools.push(res);
    //        }
    //     }
    // };
    // let mut req = Request::default();
    // req.get_parties_info_request = Some(Default::default());
    // let nodes = relay.ds.peer_store.active_nodes(None).await?;
    // let res = relay.broadcast_async(nodes, req, Some(Duration::from_secs(5))).await?;
    // for r in res {
    //     if let Ok(res) = r {
    //         if let Some(pi) = res.get_parties_info_response {
    //             for pi in pi.party_info {
    //                 if let Ok(res) = convert_party_info(&relay, &pi).await {
    //                     pools.push(res);
    //                 }
    //             }
    //         }
    //     }
    // }

    Ok(ExplorerPoolsResponse{
        pools,
    })
}



pub async fn handle_explorer_hash(hash_input: String, r: Relay, pagination: Pagination) -> RgResult<ExplorerHashSearchResponse> {
    // TODO: Math min
    let limit = Some(std::cmp::min(pagination.limit.unwrap_or(10) as i64, 100));
    let offset = Some(pagination.offset.unwrap_or(0) as i64);
    let hq = hash_query(r.clone(), hash_input.clone(), limit.clone(), offset.clone()).await?;
    let mut h = ExplorerHashSearchResponse{
        transaction: None,
        address: None,
        observation: None,
        peer: None,
        peer_node: None,
        external_txid_info: None,
    };
    let mut has_response = false;
    if let Some(ai) = &hq.address_info {
        h.address = Some(handle_address_info(ai, &r, limit, offset).await?);
        has_response = true;
    }
    if let Some(o) = &hq.observation {
        h.observation = Some(handle_observation(o, &r).await?);
        has_response = true;

    }
    if let Some(p) = &hq.peer_id_info {
        h.peer = Some(handle_peer(p, &r, false).await?);
        has_response = true;

    }
    if let Some(p) = &hq.peer_node_info {
        h.peer_node = Some(handle_peer_node(p, &r, false).await?);
        has_response = true;

    }

    if let Some(t) = hq.transaction_info {
        let detailed = convert_detailed_transaction(&r, &t).await?;
        h.transaction = Some(detailed);
        has_response = true;
    }
    if !has_response {
        if r.party_event_for_txid(&hash_input).await.is_some() {
            if let Some(pid) = r.active_party().await {
                if let Ok(ev) = convert_events(&pid, &r.node_config) {
                    if let Some(ev) = ev.iter().filter(|ev| ev.tx_hash == hash_input).next() {
                        h.external_txid_info = Some(ExternalTxidInfo {
                            external_explorer_link: "".to_string(),
                            party_address: pid.party_info.party_key.as_ref()
                                .and_then(|k| k.address().ok())
                                .and_then(|k| k.render_string().ok()).unwrap_or("".to_string()),
                            event: ev.clone(),
                        });
                    }
                }
            }
        }
        // let external_txid_info = ExternalNetworkResourcesImpl::get_external_txid_info(&r, &hash_input).await?;
        // h.external_txid_info = Some(external_txid_info);
    }
    Ok(h)
}

async fn convert_detailed_transaction(r: &Relay, t: &TransactionInfo) -> Result<DetailedTransaction, ErrorInfo> {
    let tx = t.transaction.safe_get_msg("Missing transaction but have transactionInfo")?;

    // For confirmation score, should we store that internally in the database or re-calculate it?
    let message = tx.options
        .clone()
        .and_then(|o| o.data.and_then(|d| d.message))
        .unwrap_or("".to_string());

    let mut public_to_peer: HashMap<PublicKey, (PeerTrustQueryResult, NodeSignerDetailed)> = HashMap::new();

    for s in &t.observation_proofs {
        if let (Some(p), Some(metadata), Some(observed)) = (&s.proof, &s.metadata, &s.observation_hash) {
            if let (Some(pk), Some(sig)) = (&p.public_key, &p.signature) {
                let observation_timestamp = metadata.struct_metadata.safe_get_msg("Missing struct metadata")?.time
                    .safe_get_msg("Missing time")?.clone();

                let _ = if let Some((_peer_id, _existing)) = public_to_peer.get_mut(pk) {
                    // existing
                } else {
                    let pid = r.peer_id_for_node_pk(pk).await.ok().and_then(identity);
                    let query_result = r.get_trust_of_node_as_query(pk).await?.clone();
                    let q = query_result.or(pid.map(|p| PeerTrustQueryResult { peer_id: p, trust: 1.0 }));

                    if let Some(query_result) = q {
                        let validation: f64 = metadata.validation_confidence
                            .clone()
                            .map(|v| v.label())
                            .unwrap_or(1.0) * 10.0;

                        let i33 = ValidationType::from_i32(metadata.observation_type.clone());
                        let obs_type: ValidationType = i33
                            .safe_get_msg("validationtype")?.clone();

                        let node_name = r.ds.peer_store.query_public_key_metadata(pk)
                            .await?.and_then(|m| m.node_name).unwrap_or("".to_string());

                        let ns = NodeSignerDetailed {
                            signature: sig.hex(),
                            node_id: pk.hex(),
                            node_name,
                            signed_pending_time: None,
                            observation_hash: observed.hex(),
                            observation_type: format!("{:?}", obs_type),
                            observation_timestamp: observation_timestamp.clone(),
                            validation_confidence_score: validation,
                            signed_finalized_time: None,
                        };

                        public_to_peer.insert(pk.clone(), (query_result.clone(), ns.clone()));
                    }
                };

                let state: State = State::from_i32(metadata.state.clone())
                    .safe_get_msg("state")?.clone();

                if let Some((_pk, ns)) = public_to_peer.get_mut(pk) {
                    match state {
                        State::Pending => {
                            ns.signed_pending_time = Some(observation_timestamp);
                        }
                        State::Accepted => {
                            ns.signed_finalized_time = Some(observation_timestamp);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    let mut map: HashMap<PeerId, PeerSignerDetailed> = HashMap::new();

    for (_pk, (pt, ns)) in public_to_peer.iter_mut() {
        if let Some(e) = map.get_mut(&pt.peer_id) {
            e.nodes.push(ns.clone());
        } else {
            let peer_signer = PeerSignerDetailed {
                // TODO: query peer ID from peer store
                peer_id: pt.peer_id.hex(),
                trust: pt.trust.clone() * 10.0,
                nodes: vec![ns.clone()],
            };
            map.insert(pt.peer_id.clone(), peer_signer);
        }
    }

    let mut vec = map.values().collect_vec();
    vec.sort_by(|a, b|
        a.trust.partial_cmp(&b.trust).unwrap_or(Ordering::Equal)
    );
    vec.reverse();
    let signers = vec.iter().map(|x| x.clone().clone()).collect_vec();

    let mut inputs = vec![];
    for i in &tx.inputs {
        let u = i.utxo_id.safe_get()?;
        let mut input_amount = None;
        if let Some(h) = &u.transaction_hash {
            if let Some((t, e)) = r.ds.transaction_store.query_maybe_transaction(h).await? {
                input_amount = t.outputs.get(u.output_index as usize)
                    .map(|o| o.opt_amount_typed().map(|a| a.to_fractional()).unwrap_or(0.0));
            }
        }
        let input = DetailedInput {
            transaction_hash: u.transaction_hash.clone().map(|t| t.hex()).safe_get_msg("Missing transaction hash?")?.clone(),
            output_index: u.output_index.clone(),
            address: i.address()?.render_string()?,
            input_amount,
        };
        inputs.push(input);
    }
    let mut outputs = vec![];
    let tx_hash = tx.hash_or();
    for (i, o) in tx.outputs.iter().enumerate() {

        let utxo_e = o.utxo_entry(&tx_hash, i as i64, 0);
        let u = utxo_e.utxo_id.safe_get_msg("Missing utxo id")?;

        let children = r.ds.utxo.utxo_children(u).await?
            .iter().map(|(h, i)| {
                UtxoChild {
                    used_by_tx: h.hex(),
                    used_by_tx_input_index: i.clone() as i32,
                    status: "Confirmed".to_string(),
                }
            }).collect_vec();

        let output = DetailedOutput {
            output_index: i.clone() as i32,
            address: o.address.safe_get()?.render_string()?,
            available: t.valid_utxo_index.contains(&(i as i32)),
            amount: o.opt_amount_typed().map(|a| a.to_fractional()).unwrap_or(0.0),
            children,
            is_swap: o.is_swap(),
            is_liquidity: o.is_stake(),
        };
        outputs.push(output);
    }

    // TODO: Make this over Vec<ObservationProof> Instead
    let mut submit_response = SubmitTransactionResponse::default();
    let mut query_transaction_response = QueryTransactionResponse::default();
    query_transaction_response.observation_proofs = t.observation_proofs.clone().iter().map(|o| o.clone()).collect_vec();
    submit_response.query_transaction_response = Some(query_transaction_response);
    submit_response.transaction = Some(tx.clone());
    let counts = submit_response.count_unique_by_state()?;

    let num_pending_signers = counts.get(&(State::Pending as i32)).unwrap_or(&0).clone() as i64;
    let num_accepted_signers = counts.get(&(State::Accepted as i32)).unwrap_or(&0).clone() as i64;

    let mut detailed = DetailedTransaction {
        info: brief_transaction(tx)?,
        confirmation_score: 1.0,
        acceptance_score: 1.0,
        message,
        num_pending_signers,
        num_accepted_signers,
        accepted: t.accepted,
        signers,
        inputs,
        outputs,
        rejection_reason: t.rejection_reason.clone(),
        signable_hash: tx.signable_hash().hex(),
        raw_transaction: tx.clone(),
        remainder_amount: CurrencyAmount::from(tx.remainder_amount()).to_fractional(),
        party_related_info: None,
    };


    if r.party_event_for_txid(&detailed.info.hash).await.is_some() {
        if let Some(pid) = r.active_party().await {
            if let Ok(ev) = convert_events(&pid, &r.node_config) {
                if let Some(ev) = ev.iter().filter(|ev| ev.tx_hash == detailed.info.hash).next() {
                    detailed.party_related_info = Some(PartyRelatedInfo {
                        party_address: pid.party_info.party_key.as_ref()
                            .and_then(|k| k.address().ok())
                            .and_then(|k| k.render_string().ok()).unwrap_or("".to_string()),
                        event: Some(ev.clone()),
                    });
                }
            }
        }
    }

    Ok(detailed)
}


// TODO Make trait implicit
fn brief_transaction(tx: &Transaction) -> RgResult<BriefTransaction> {
    Ok(BriefTransaction {
        hash: tx.hash_or().hex(),
        from: tx.first_input_address()
            .and_then(|a| a.render_string().ok())
            .unwrap_or("".to_string()),
        to: tx.first_output_address_non_input_or_fee().safe_get_msg("Missing output address")?.render_string()?,
        amount: tx.total_output_amount_float(),
        fee: tx.fee_amount(),
        bytes: tx.proto_serialize().len() as i64,
        timestamp: tx.struct_metadata.clone().and_then(|s| s.time).safe_get_msg("Missing tx timestamp")?.clone(),
        first_amount: tx.first_output_amount().safe_get_msg("Missing first output amount")?.clone(),
        is_test: tx.is_test(),
    })
}


// #[tracing::instrument()]
pub async fn handle_explorer_recent(r: Relay, is_test: Option<bool>) -> RgResult<RecentDashboardResponse> {

    // r.node_config.self_client().metrics()

    let start = current_time_millis_i64();
    let recent = r.ds.transaction_store.query_recent_transactions(Some(10), is_test).await?;
    trace!("Dashboard query time elapsed: {:?}", current_time_millis_i64() - start);
    let mut recent_transactions = Vec::new();
    for tx in recent {
        let brief_tx = brief_transaction(&tx)?;
        recent_transactions.push(brief_tx);
    }
    trace!("Brief transaction build time elapsed: {:?}", current_time_millis_i64() - start);
    // // TODO: Rename this
    // let total_accepted_transactions =
    //     r.ds.transaction_store.count_total_transactions().await?;
    // trace!("count_total_transactions time elapsed: {:?}", current_time_millis_i64() - start);

    let (num_active_peers, active_peers_abridged) = load_active_peers_info(&r, start).await?;
    trace!("active nodes and peers done time elapsed: {:?}", current_time_millis_i64() - start);


    let obs = r.ds.observation.recent_observation(
        Some(10),
    ).await?;
    trace!("observation query: {:?}", current_time_millis_i64() - start);

    let mut recent_observations = vec![];
    for i in obs[0..10.min(obs.len())].iter() {
        let o = handle_observation(&i, &r).await?;
        recent_observations.push(o);
    }
    trace!("observation format: {:?}", current_time_millis_i64() - start);


    let client = r.node_config.self_client().client_wrapper();
    let tables = client.table_sizes_map().await?;
    let metrics = client.metrics_map().await?;

    Ok(RecentDashboardResponse {
        recent_transactions,
        total_accepted_transactions: metrics.get("redgold_transaction_accepted_total")
            .and_then(|v| v.parse::<i64>().ok()).unwrap_or(0),
        size_transactions_gb: tables.get("transactions")
            .map(|v| (v.clone() as f64) / (1024*1024*1024) as f64).unwrap_or(0.0),
        total_accepted_utxos: metrics.get("redgold_utxo_total")
            .and_then(|v| v.parse::<i64>().ok()).unwrap_or(0),
        size_utxos_gb: tables.get("utxo")
            .map(|v| (v.clone() as f64) / (1024*1024*1024) as f64).unwrap_or(0.0),
        total_accepted_observations: metrics.get("redgold_observation_total")
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0),
        size_observations_gb: tables.get("observation")
            .map(|v| (v.clone() as f64) / (1024*1024*1024) as f64).unwrap_or(0.0),
        total_distinct_utxo_addresses: metrics.get("redgold_utxo_distinct_addresses")
            .and_then(|v| v.parse::<i64>().ok()).unwrap_or(0),
        num_active_peers,
        active_peers_abridged,
        recent_observations
    })
}

async fn load_active_peers_info(r: &Relay, start: i64) -> Result<(i64, Vec<DetailedPeer>), ErrorInfo> {
    let peers = r.ds.peer_store.active_nodes(None).await?;
    trace!("active nodes ds query elapsed: {:?}", current_time_millis_i64() - start);

    let num_active_peers = (peers.len() as i64) + 1;

    let pks = peers[0..9.min(peers.len())].to_vec();
    let mut active_peers_abridged = vec![];
    active_peers_abridged.push(
        handle_peer(&r.peer_id_info().await?, &r, true).await?
    );

    trace!("active nodes first handle peer elapsed: {:?}", current_time_millis_i64() - start);

    for pk in &pks {
        let option = r.ds.peer_store.peer_id_for_node_pk(pk).await?;
        if let Some(pid) = option {
            let option1 = r.ds.peer_store.query_peer_id_info(&pid).await?;
            if let Some(pid_info) = option1 {
                if let Some(p) = handle_peer(&pid_info, &r, true).await.ok() {
                    active_peers_abridged.push(p);
                }
            }
        }
    }

    active_peers_abridged = active_peers_abridged.iter().filter(|p| !p.nodes.is_empty()).cloned().collect_vec();
    Ok((num_active_peers, active_peers_abridged))
}

pub async fn handle_explorer_swap(relay: Relay) -> RgResult<Option<AddressPoolInfo>> {
    get_address_pool_info(relay).await
}

#[ignore]
#[tokio::test]
async fn debug_peers_load() {
    let r = Relay::dev_default().await;
    let start = current_time_millis_i64();
    let res = load_active_peers_info(&r, start).await.expect("failed to load peers");
    println!("Peers: {}", res.1.json_or());

}