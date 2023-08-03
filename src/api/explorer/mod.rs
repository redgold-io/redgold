pub mod server;

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use eframe::egui::accesskit::Role::Math;
use itertools::Itertools;
use rocket::form::FromForm;
use redgold_schema::{ProtoSerde, RgResult, SafeBytesAccess, SafeOption, WithMetadataHashable};
use crate::api::hash_query::hash_query;
use crate::core::relay::Relay;
use serde::{Serialize, Deserialize};
use redgold_data::peer::PeerTrustQueryResult;
use redgold_schema::structs::{AddressInfo, ErrorInfo, HashType, NetworkEnvironment, NodeType, Observation, ObservationMetadata, PeerId, PeerIdInfo, PeerNodeInfo, PublicKey, QueryTransactionResponse, State, SubmitTransactionResponse, Transaction, TrustLabel, UtxoEntry, ValidationType};
use strum_macros::EnumString;
use warp::get;
use redgold_schema::transaction::{rounded_balance, rounded_balance_i64};
use crate::api::public_api::Pagination;
use crate::multiparty::watcher::{BidAsk, DepositWatcherConfig};
use crate::util;
use crate::util::address_external::ToBitcoinAddress;

#[derive(Serialize, Deserialize)]
pub struct HashResponse {
    pub hash: String,
    pub height: u64,
    pub timestamp: u64,
    pub transactions: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct BriefTransaction {
    pub hash: String,
    pub from: String,
    pub to: String,
    pub amount: f64,
    pub fee: f64,
    pub bytes: i64,
    pub timestamp: i64,
    pub first_amount: f64,
}



#[derive(Serialize, Deserialize, Clone)]
pub struct PeerSignerDetailed {
    pub peer_id: String,
    pub nodes: Vec<NodeSignerDetailed>,
    pub trust: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NodeSignerDetailed {
    pub signature: String,
    pub node_id: String,
    pub signed_pending_time: Option<i64>,
    pub signed_finalized_time: Option<i64>,
    pub observation_hash: String,
    pub observation_type: String,
    pub observation_timestamp: i64,
    pub validation_confidence_score: f64,
}


#[derive(Serialize, Deserialize)]
pub struct DetailedInput {
    pub transaction_hash: String,
    pub output_index: i64,
    pub address: String
}

#[derive(Serialize, Deserialize)]
pub struct DetailedOutput {
    pub output_index: i32,
    pub address: String,
    pub available: bool,
    pub amount: f64,
}


#[derive(Serialize, Deserialize)]
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
}


#[derive(Serialize, Deserialize)]
pub struct AddressPoolInfo {
    rdg_address: String,
    rdg_balance: f64,
    btc_address: String,
    btc_balance: f64,
    bid_ask: BidAsk,
}

#[derive(Serialize, Deserialize)]
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


#[derive(Serialize, Deserialize)]
pub struct BriefUtxoEntry {
    pub transaction_hash: String,
    pub output_index: i64,
    pub amount: f64,
    pub time: i64
}



#[derive(Serialize, Deserialize)]
pub struct DetailedObservationMetadata {
    pub observed_hash: String,
    pub observed_hash_type: String,
    pub validation_type: String,
    pub state: String,
    pub validation_confidence: f64,
    pub time: i64,
    pub metadata_hash: String,
}

#[derive(Serialize, Deserialize)]
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
    pub parent_hash: String
}



#[derive(Serialize, Deserialize)]
pub struct DetailedTrust {
    pub peer_id: String,
    pub trust: f64,
}


#[derive(Serialize, Deserialize)]
pub struct DetailedPeer {
    pub peer_id: String,
    pub public_key: String,
    pub signature: String,
    pub nodes: Vec<DetailedPeerNode>,
    pub trust: Vec<DetailedTrust>
}

#[derive(Serialize, Deserialize)]
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
    pub alias: String,
    pub name: String,
    pub peer_id: String,
    pub nat_restricted: bool,
    pub network_environment: String,
}


#[derive(Serialize, Deserialize)]
pub struct ExplorerHashSearchResponse {
    pub transaction: Option<DetailedTransaction>,
    pub address: Option<DetailedAddress>,
    pub observation: Option<DetailedObservation>,
    pub peer: Option<DetailedPeer>,
    pub peer_node: Option<DetailedPeerNode>,
}

#[derive(Serialize, Deserialize)]
pub struct RecentDashboardResponse {
    pub recent_transactions: Vec<BriefTransaction>,
    total_accepted_transactions: i64,
    num_active_peers: i64,
    pub active_peers_abridged: Vec<DetailedPeer>,
    pub recent_observations: Vec<DetailedObservation>
}

pub fn convert_utxo(u: &UtxoEntry) -> RgResult<BriefUtxoEntry> {
    Ok(BriefUtxoEntry {
        transaction_hash: u.transaction_hash.safe_get_msg("Missing transaction hash")?.hex(),
        output_index: u.output_index.clone(),
        amount: rounded_balance(u.amount()),
        time: u.time.clone(),
    })
}

pub async fn get_address_pool_info(r: Relay) -> RgResult<Option<AddressPoolInfo>> {

    let res: Option<DepositWatcherConfig> = r.ds.config_store.get_json::<DepositWatcherConfig>("deposit_watcher_config").await?;
    let res = match res {
        None => {
            None
        }
        Some(d) => {
            let a = d.deposit_allocations.get(0).safe_get_msg("Missing deposit alloc")?.clone();
            let btc_swap_address = a.key.to_bitcoin_address_network(r.node_config.network.clone())?;
            let btc_amount = (a.balance_btc as f64) / 1e8;
            let rdg_amount = (a.balance_rdg as f64) / 1e8;
            Some(AddressPoolInfo {
                rdg_address: a.key.address()?.render_string()?,
                rdg_balance: rdg_amount,
                btc_address: btc_swap_address,
                btc_balance: btc_amount,
                bid_ask: d.bid_ask.clone(),
            })
        }
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
        .filter(|p| p.btc_address == address_str || p.rdg_address == address_str);

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
        format!("{:?}", State::from_i32(om.state.safe_get()?.clone()).safe_get()?.clone()),
        validation_confidence: om.validation_confidence.as_ref()
            .map(|l| l.label() * 10.0)
            .unwrap_or(10.0),
        time: om.struct_metadata.safe_get()?.time.safe_get()?.clone(),
        metadata_hash: om.struct_metadata.safe_get()?.hash.safe_get()?.hex(),
    })
}

pub async fn handle_observation(o: &Observation, r: &Relay) -> RgResult<DetailedObservation> {

    Ok(DetailedObservation {
        merkle_root: o.merkle_root.safe_get()?.hex(),
        observations: o.observations.iter()
            .map(|om| convert_observation_metadata(om))
            .collect::<RgResult<Vec<DetailedObservationMetadata>>>()?,
        public_key: o.proof.safe_get()?.public_key.safe_get()?.hex_or(),
        signature: hex::encode(o.proof.safe_get()?.signature.safe_get()?.bytes.safe_bytes()?),
        time: o.struct_metadata.safe_get()?.time.safe_get()?.clone(),
        hash: o.struct_metadata.safe_get()?.hash.safe_get()?.hex(),
        signable_hash: o.signable_hash().hex(),
        salt: o.salt.clone(),
        height: o.height.clone(),
        parent_hash: o.parent_hash.as_ref().map(|h| h.hex()).unwrap_or("".to_string()),
    })
}

pub fn convert_trust(trust: &TrustLabel) -> RgResult<DetailedTrust> {
    Ok(DetailedTrust{
        peer_id: hex::encode(&trust.peer_id),
        trust: trust.trust_data.get(0).safe_get()?.label(),
    })
}

pub async fn handle_peer(p: &PeerIdInfo, r: &Relay) -> RgResult<DetailedPeer> {
    let pd = p.latest_peer_transaction.safe_get()?.peer_data()?;
    let mut nodes = vec![];
    for pni in &p.peer_node_info {
        nodes.push(handle_peer_node(pni, &r).await?);
    }
    Ok(DetailedPeer {
        peer_id: hex::encode(pd.peer_id.safe_get()?.peer_id.safe_get()?.bytes.safe_bytes()?),
        // TODO: From transaction, should include address and latest input pk?
        // or do the merkle proofs contain this?
        public_key: "".to_string(), //pd.proof.safe_get()?.public_key.safe_get()?.hex_or(),
        signature: "".to_string(), // hex::encode(pd.proof.safe_get()?.signature.safe_get()?.bytes.safe_bytes()?),
        nodes,
        trust: pd.labels.iter().map(|l| convert_trust(l))
            .collect::<RgResult<Vec<DetailedTrust>>>()?,
    })
}

pub async fn handle_peer_node(p: &PeerNodeInfo, r: &Relay) -> RgResult<DetailedPeerNode> {
    let nmd = p.latest_node_transaction.safe_get()?.node_metadata()?;
    let vi = nmd.version_info.clone().safe_get()?.clone();
    Ok(DetailedPeerNode{
        external_address: nmd.external_address.clone(),
        public_key: nmd.public_key.safe_get()?.hex()?,
        node_type:  format!("{:?}", NodeType::from_i32(nmd.node_type.safe_get()?.clone()).safe_get()?.clone()),
        executable_checksum: vi.executable_checksum.clone(),
        commit_hash: vi.commit_hash.unwrap_or("".to_string()),
        next_executable_checksum: vi.next_executable_checksum.clone().unwrap_or("".to_string()),
        next_upgrade_time: vi.next_upgrade_time.clone(),
        utxo_distance: nmd.partition_info.as_ref()
            .and_then(|p| p.utxo_distance)
            .map(|d| (d as f64) / 1000 as f64) // TODO: use a function for this
            .unwrap_or(1.0),
        port_offset: nmd.port_offset.unwrap_or(0),
        alias: nmd.alias.unwrap_or("".to_string()),
        name: nmd.name.unwrap_or("".to_string()),
        peer_id: nmd.peer_id.as_ref()
            .and_then(|p| p.peer_id.safe_get().ok())
            .and_then(|p| p.bytes.safe_bytes().ok())
            .map(|p| hex::encode(p)).unwrap_or("".to_string()),
        nat_restricted: nmd.nat_restricted.unwrap_or(false),
        network_environment:
        format!("{:?}", NetworkEnvironment::from_i32(nmd.network_environment.clone()).safe_get()?.clone()),
    })
}

pub async fn handle_explorer_hash(hash_input: String, r: Relay, pagination: Pagination) -> RgResult<ExplorerHashSearchResponse> {
    // TODO: Math min
    let limit = Some(std::cmp::min(pagination.limit.unwrap_or(10) as i64, 100));
    let offset = Some(pagination.offset.unwrap_or(0) as i64);
    let hq = hash_query(r.clone(), hash_input, limit.clone(), offset.clone()).await?;
    let mut h = ExplorerHashSearchResponse{
        transaction: None,
        address: None,
        observation: None,
        peer: None,
        peer_node: None,
    };
    if let Some(ai) = &hq.address_info {
        h.address = Some(handle_address_info(ai, &r, limit, offset).await?);
    }
    if let Some(o) = &hq.observation {
        h.observation = Some(handle_observation(o, &r).await?);
    }
    if let Some(p) = &hq.peer_id_info {
        h.peer = Some(handle_peer(p, &r).await?);
    }
    if let Some(p) = &hq.peer_node_info {
        h.peer_node = Some(handle_peer_node(p, &r).await?);
    }

    if let Some(t) = hq.transaction_info {
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

                    let _ = if let Some((peer_id, existing)) = public_to_peer.get_mut(pk) {
                        // existing
                    } else {
                        let query_result = r.ds.peer_store
                            .node_peer_id_trust(pk).await?
                            .unwrap_or({
                                let empty = PeerTrustQueryResult {
                                    peer_id: PeerId::from_bytes(vec![]),
                                    trust: 0.0,
                                };
                                let result = if &r.node_config.clone().public_key() == pk {
                                    PeerTrustQueryResult {
                                        peer_id: r.node_config.peer_id(),
                                        trust: 1.0,
                                    }
                                } else {
                                    empty
                                };
                                result
                            });

                        let validation: f64 = metadata.validation_confidence
                            .clone()
                            .map(|v| v.label())
                            .unwrap_or(1.0) * 10.0;

                        let i33 = ValidationType::from_i32(metadata.observation_type.clone());
                        let obs_type: ValidationType = i33
                            .safe_get_msg("validationtype")?.clone();

                        let ns = NodeSignerDetailed {
                            signature: hex::encode(sig.bytes.safe_bytes()?),
                            node_id: pk.hex_or(),
                            signed_pending_time: None,
                            observation_hash: observed.hex(),
                            observation_type: format!("{:?}", obs_type),
                            observation_timestamp: observation_timestamp.clone(),
                            validation_confidence_score: validation,
                            signed_finalized_time: None,
                        };

                        public_to_peer.insert(pk.clone(), (query_result.clone(), ns.clone()));
                    };

                    let state: State = State::from_i32(metadata.state.safe_get_msg("Missing state")?.clone())
                        .safe_get_msg("state")?.clone();

                    if let Some((pk, ns)) = public_to_peer.get_mut(pk) {
                        match state {
                            State::Pending => {
                                ns.signed_pending_time = Some(observation_timestamp);
                            }
                            State::Finalized => {
                                ns.signed_finalized_time = Some(observation_timestamp);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let mut map: HashMap<PeerId, PeerSignerDetailed> = HashMap::new();

        for (pk, (pt, ns)) in public_to_peer.iter_mut() {
            if let Some(e) = map.get_mut(&pt.peer_id) {
                e.nodes.push(ns.clone());
            } else {
                let peer_signer = PeerSignerDetailed {
                    // TODO: query peer ID from peer store
                    peer_id: hex::encode(pt.peer_id.peer_id.safe_get()?.bytes.safe_bytes()?),
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
            let input = DetailedInput{
                transaction_hash: i.transaction_hash.clone().map(|t| t.hex()).safe_get_msg("Missing transaction hash?")?.clone(),
                output_index: i.output_index.clone(),
                address: i.address()?.render_string()?,
            };
            inputs.push(input);
        }
        let mut outputs = vec![];
        for (i, o) in tx.outputs.iter().enumerate() {
            let output = DetailedOutput{
                output_index: i.clone() as i32,
                address: o.address.safe_get()?.render_string()?,
                available: t.valid_utxo_index.contains(&(i as i32)),
                amount: o.rounded_amount(),
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
        let num_accepted_signers = counts.get(&(State::Finalized as i32)).unwrap_or(&0).clone() as i64;
        let mut detailed = DetailedTransaction{
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
            rejection_reason: t.rejection_reason,
            signable_hash: tx.signable_hash().hex(),
        };
        h.transaction = Some(detailed)
    }
    Ok(h)
}


fn brief_transaction(tx: &Transaction) -> RgResult<BriefTransaction> {
    Ok(BriefTransaction {
        hash: tx.hash_or().hex(),
        from: tx.first_input_address()
            .and_then(|a| a.render_string().ok())
            .unwrap_or("".to_string()),
        to: tx.first_output_address().safe_get_msg("Missing output address")?.render_string()?,
        amount: tx.total_output_amount_float(),
        fee: 0f64, // Replace with find fee address?
        bytes: tx.proto_serialize().len() as i64,
        timestamp: tx.struct_metadata.clone().and_then(|s| s.time).safe_get_msg("Missing tx timestamp")?.clone(),
        first_amount: tx.first_output_amount().safe_get_msg("Missing first output amount")?.clone(),
    })
}


pub async fn handle_explorer_recent(r: Relay) -> RgResult<RecentDashboardResponse>{
    let recent = r.ds.transaction_store.query_recent_transactions(Some(10)).await?;
    let mut recent_transactions = Vec::new();
    for tx in recent {
        let brief_tx = brief_transaction(&tx)?;
        recent_transactions.push(brief_tx);
    }
    let total_accepted_transactions =
        r.ds.transaction_store.count_total_accepted_transactions().await?;
    let peers = r.ds.peer_store.active_nodes(None).await?;
    let num_active_peers = (peers.len() as i64) + 1;

    let pks = peers[0..9.min(peers.len())].to_vec();
    let mut active_peers_abridged = vec![];
    active_peers_abridged.push(
        handle_peer(&r.peer_id_info().await?, &r).await?
    );
    for pk in pks {
        if let Some(pid) = r.ds.peer_store.query_public_key_node(pk).await?
            .and_then(|p| p.latest_peer_transaction)
            .and_then(|p| p.peer_data().ok())
            .and_then(|p| p.peer_id) {
            if let Some(pid_info) = r.ds.peer_store.query_peer_id_info(&pid).await? {
                if let Some(p) = handle_peer(&pid_info, &r).await.ok() {
                    active_peers_abridged.push(p);
                }
            }
        }
    }


    let obs = r.ds.observation.recent_observation(
        Some(10),
    ).await?;

    let mut recent_observations = vec![];
    for i in obs[0..10.min(obs.len())].iter() {
        let o = handle_observation(&i, &r).await?;
        recent_observations.push(o);
    }

    Ok(RecentDashboardResponse {
        recent_transactions,
        total_accepted_transactions,
        num_active_peers,
        active_peers_abridged,
        recent_observations
    })
}

pub async fn handle_explorer_swap(relay: Relay) -> RgResult<Option<AddressPoolInfo>> {
    get_address_pool_info(relay).await
}