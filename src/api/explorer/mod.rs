pub mod server;
pub mod debug_test;

use crate::api::faucet::faucet_request;
use crate::api::hash_query::hash_query;
use crate::api::public_api::{Pagination, TokenParam};
use crate::core::relay::Relay;
use crate::integrations::external_network_resources::ExternalNetworkResourcesImpl;
use crate::node_config::ApiNodeConfig;
use crate::party::price_query::PriceDataPointQueryImpl;
// use crate::multiparty_gg20::watcher::{DepositWatcher, DepositWatcherConfig};
use crate::util;
use crate::util::current_time_millis_i64;
use eframe::egui::accesskit::Role::Math;
use futures::TryFutureExt;
use itertools::Itertools;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_data::peer::PeerTrustQueryResult;
use redgold_keys::address_external::ToBitcoinAddress;
use redgold_keys::address_support::AddressSupport;
use redgold_keys::external_tx_support::ExternalTxSupport;
use redgold_keys::proof_support::PublicKeySupport;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::explorer::{AddressPoolInfo, BriefTransaction, BriefUtxoEntry, DetailedAddress, DetailedInput, DetailedObservation, DetailedObservationMetadata, DetailedOutput, DetailedPartyEvent, DetailedPeer, DetailedPeerNode, DetailedTransaction, DetailedTrust, ExplorerFaucetResponse, ExplorerHashSearchResponse, ExplorerPoolInfoResponse, ExplorerPoolsResponse, ExternalTxidInfo, NodeSignerDetailed, PartyRelatedInfo, PeerSignerDetailed, PoolMember, RecentDashboardResponse, SwapStatus, TransactionSwapInfo, UtxoChild};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::party::address_event::AddressEvent;
use redgold_schema::party::central_price::CentralPricePair;
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::party::price_volume::PriceVolume;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::PartyInfoAbridged;
use redgold_schema::structs::{Address, AddressInfo, CurrencyAmount, ErrorInfo, FaucetRequest, FaucetResponse, HashType, NetworkEnvironment, NodeType, Observation, ObservationMetadata, PeerId, PeerIdInfo, PeerNodeInfo, PublicKey, QueryTransactionResponse, Request, State, SubmitTransactionResponse, SupportedCurrency, Transaction, TransactionInfo, TrustRatingLabel, UtxoEntry, ValidationType};
use redgold_schema::transaction::{rounded_balance, rounded_balance_i64};
use redgold_schema::tx::currency_amount::RenderCurrencyAmountDecimals;
use redgold_schema::util::times::ToTimeString;
use redgold_schema::{error_info, explorer, RgResult, SafeOption};
use rocket::form::FromForm;
use rocket::yansi::Paint;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::convert::identity;
use std::hash::Hash;
use std::net::SocketAddr;
use std::time::Duration;
use strum_macros::EnumString;
use tokio::time::Instant;
use tracing::info;
use tracing::trace;
use warp::get;
// use crate::party::bid_ask::BidAsk;


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
            let overall_staking_balances = format_currency_balances(pe.staking_balances(&vec![], Some(true), Some(true)));
            let portfolio_staking_balances = format_currency_balances(pe.staking_balances(&vec![], Some(true), Some(false)));
            let amm_staking_balances = format_currency_balances(pe.staking_balances(&vec![], Some(false), Some(true)));

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
                overall_staking_balances,
                portfolio_staking_balances,
                amm_staking_balances
            }))
        };


    }
    Ok(None)
}

pub fn format_currency_balances(balances: HashMap<SupportedCurrency, CurrencyAmount>) -> Vec<(String, String)> {
    balances.iter().map(|(k, v)| {
        (format!("{:?}", k), format!("{:.8}", v.to_fractional().to_string()))
    }).collect::<Vec<(String, String)>>()
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
    let address_str = a.render_string()?;
    let outgoing_from = Some(address_str.clone());

    let recent: Vec<Transaction> = ai.recent_transactions.clone();
    let incoming_transactions = r.ds.transaction_store.get_filter_tx_for_address(
        &a, limit.unwrap_or(10), offset.unwrap_or(0), true
    ).await?.iter().map(|u| explorer::brief_transaction(&u, outgoing_from.clone(), None)).collect::<RgResult<Vec<BriefTransaction>>>()?;
    let outgoing_transactions = r.ds.transaction_store.get_filter_tx_for_address(
        &a, limit.unwrap_or(10), offset.unwrap_or(0), false
    ).await?.iter().map(|u| explorer::brief_transaction(&u, outgoing_from.clone(), None)).collect::<RgResult<Vec<BriefTransaction>>>()?;

    let incoming_count = r.ds.transaction_store.get_count_filter_tx_for_address(&a, true).await?;
    let outgoing_count = r.ds.transaction_store.get_count_filter_tx_for_address(&a, false).await?;
    let total_count = incoming_count.clone() + outgoing_count.clone();

    let address_pool_info = get_address_pool_info(r.clone()).await?
        .filter(|p| p.addresses.values().collect_vec().contains(&&address_str));

    let detailed = DetailedAddress {
        address: address_str,
        balance: rounded_balance_i64(ai.balance.clone()),
        total_utxos: ai.utxo_entries.len() as i64,
        recent_transactions: recent.iter().map(|u| explorer::brief_transaction(&u, outgoing_from.clone(), None)).collect::<RgResult<Vec<BriefTransaction>>>()?,
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
        if let Some((ae, pid)) = r.party_event_for_txid(&hash_input).await {
            if let Ok(ev) = convert_events(&pid, &r.node_config) {
                if let Some(ev) = ev.iter().filter(|ev| ev.tx_hash == hash_input).next() {
                    let mut external_txid_info = ExternalTxidInfo {
                        external_explorer_link: "".to_string(),
                        party_address: pid.party_info.party_key.as_ref()
                            .and_then(|k| k.address().ok())
                            .and_then(|k| k.render_string().ok()).unwrap_or("".to_string()),
                        event: ev.clone(),
                        swap_info: None,
                    };
                    external_txid_info.swap_info = check_for_external_network_swap_info(None, Some(external_txid_info.clone()), &r).await?;
                    h.external_txid_info = Some(external_txid_info);
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
        info: explorer::brief_transaction(tx, None, None)?,
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
        swap_info: None,
    };


    if let Some((ae, pid)) = r.party_event_for_txid(&detailed.info.hash).await {
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
        derive_tx_swap_info(&tx, r).await.map(|tsi| {
            detailed.swap_info = Some(tsi);
        });
    }

    Ok(detailed)
}

pub async fn check_for_external_network_swap_info(a: Option<Address>, opt_ext: Option<ExternalTxidInfo>, r: &Relay) -> RgResult<Option<TransactionSwapInfo>> {
    let mut tsi = TransactionSwapInfo::default();
    let mut address = "".to_string();
    if let Some(opt_ext) = opt_ext.as_ref() {
        address = opt_ext.event.other_address.clone();
    } else if let Some(a) = a.as_ref() {
        address = a.render_string()?;
    }
    for (k, v) in r.external_network_shared_data.clone_read().await.iter() {
        if let Some(pev) = v.party_events.as_ref() {
            for (of, init, end) in pev.fulfillment_history.iter() {
                match init {
                    AddressEvent::External(e) => {
                        if opt_ext.clone().map(|o| o.event.tx_hash == e.tx_id)
                            .unwrap_or(false) ||
                        a.clone().map(|aa| aa == e.other_address_typed().unwrap()).unwrap_or(false) {
                            let amount = e.currency_amount();
                            if let Some(p) = e.price_usd {
                                tsi.swap_input_amount_usd = (amount.to_fractional() * p).render_currency_amount_2_decimals();
                            }
                            if let Ok(Some(p)) = r.ds.price_time
                                .max_time_price_by(e.currency, current_time_millis_i64()).await {
                                tsi.swap_input_amount_usd_now = (amount.to_fractional() * p).render_currency_amount_2_decimals();
                            }
                            tsi.swap_input_amount = amount.render_8_decimals();
                            tsi.swap_status = SwapStatus::Complete;
                            tsi.is_fulfillment = false;
                            tsi.party_address = k.address().ok()
                                .and_then(|a| a.render_string().ok()).unwrap_or_default();
                            tsi.output_currency = SupportedCurrency::Redgold;
                            tsi.swap_destination_address = address.clone();
                            match end {
                                AddressEvent::Internal(txo) => {
                                    tsi.fulfillment_tx_hash = Some(txo.tx.hash_hex());
                                    tsi.is_request = true;
                                    tsi.swap_output_amount = Some(of.fulfilled_currency_amount().render_8_decimals());
                                    if let Some(p) = pev.get_rdg_max_bid_usd_estimate_at(current_time_millis_i64()) {
                                        tsi.swap_input_amount_usd_now = (amount.to_fractional() * p).render_currency_amount_2_decimals();
                                    }
                                    if let Some(p) = pev.get_rdg_max_bid_usd_estimate_at(of.event_time) {
                                        tsi.swap_output_amount_usd = Some((
                                            of.fulfilled_currency_amount().to_fractional() * p).render_currency_amount_2_decimals());
                                    }
                                }
                                _ => {

                                }
                            }
                            return Ok(Some(tsi))
                        }
                    }
                    AddressEvent::Internal(_) => {}
                }
            }
        }
    };
    Ok(None)
}

pub async fn derive_tx_swap_info(tx: &Transaction, r: &Relay) -> Option<TransactionSwapInfo> {
    let mut tsi = TransactionSwapInfo::default();
    // This must be a swap to an external currency.
    if let Some((sr, sa, party)) = tx.swap_request_and_amount_and_party_address() {
        tsi.swap_destination_address = sr.destination.as_ref().and_then(|d| d.render_string().ok()).unwrap_or_default();
        tsi.is_request = true;
        tsi.is_fulfillment = false;
        tsi.input_currency = SupportedCurrency::Redgold;
        if let Some(c) = sr.destination.as_ref()
            .map(|d| d.currency_or()) {
            tsi.output_currency = c;
        }
        tsi.party_address = party.render_string().unwrap_or_default();
        tsi.swap_input_amount = format!("{:8}", sa.to_fractional().to_string());

        if let Some(p) = r.get_party_by_address(party).await {

            if let Some(pev) = p.party_events.as_ref() {

                if let Some(p) = pev.get_rdg_max_bid_usd_estimate_at(current_time_millis_i64()) {
                    tsi.swap_input_amount_usd_now = (sa.to_fractional() * p).render_currency_amount_2_decimals();
                }
                pev.central_prices.get(&tsi.output_currency).map(|cp| {
                    tsi.swap_output_amount = Some((cp.min_bid * sa.to_fractional()).render_currency_amount_8_decimals());
                    tsi.swap_output_amount_usd = Some((cp.min_bid_estimated * sa.to_fractional()).render_currency_amount_2_decimals());
                });
                tsi.swap_input_amount_usd = sa.to_fractional().to_string();

                if let Some((off, init, end)) = pev.find_fulfillment_of(tx.hash_hex()) {
                    tsi.fulfillment_tx_hash = Some(end.identifier());
                    tsi.swap_status = SwapStatus::Complete;
                };
            }
        }
        Some(tsi)
        // This MUST be a fulfillment of Redgold currency! Because here we have a transaction
        // in Redgold, with some amount, and the initiating event must be an external deposit to
        // party address
    } else if let Some((f, a, destination, origin)) = tx.swap_fulfillment_amount_and_destination_and_origin() {
        if let Some(p) = r.get_party_events_by_address(&origin).await {
            if let Some((of, e1, e2)) = p.find_request_fulfilled_by(tx.hash_hex()) {
                tsi.swap_status = SwapStatus::Complete;
                tsi.output_currency = SupportedCurrency::Redgold;
                if let Some(c) = e1.external_currency() {
                    let price = r.ds.price_time.max_time_price_by(c, of.event_time).await.ok().flatten();
                    let price_now = r.ds.price_time.max_time_price_by(c, current_time_millis_i64()).await.ok().flatten();
                    if let Some(p) = price {
                        tsi.swap_input_amount_usd = (a.to_fractional() * p).render_currency_amount_2_decimals();
                    }
                    if let Some(p) = price_now {
                        tsi.swap_input_amount_usd_now = (a.to_fractional() * p).render_currency_amount_2_decimals();
                    }
                    tsi.input_currency = c
                }
                // tsi.swap_fee
                tsi.party_address = origin.render_string().unwrap_or_default();
                tsi.is_request = false;
                tsi.exchange_rate = Some(format!("{:.2}", (of.order_amount as f64) / (of.fulfilled_amount as f64)));
                tsi.fulfillment_tx_hash = Some(e2.identifier());
                tsi.input_currency = e1.external_currency().unwrap_or(SupportedCurrency::Redgold);
                tsi.request_tx_hash = Some(e1.identifier());
                tsi.is_fulfillment = true;
                tsi.swap_destination_address = destination.render_string().unwrap_or_default();
                tsi.swap_output_amount = Some(a.to_fractional().render_currency_amount_8_decimals());
                let price_output = p.get_rdg_max_bid_usd_estimate_at(of.event_time);
                if let Some(p) = price_output {
                    tsi.swap_output_amount_usd = Some((a.to_fractional() * p).render_currency_amount_2_decimals());
                }
                let price_output = p.get_rdg_max_bid_usd_estimate_at(util::current_time_millis_i64());
                if let Some(p) = price_output {
                    tsi.swap_output_amount_usd = Some((a.to_fractional() * p).render_currency_amount_2_decimals());
                }
            }
        }
        Some(tsi)
    } else {
        None
    }
}


// #[tracing::instrument()]
pub async fn handle_explorer_recent(r: Relay, is_test: Option<bool>) -> RgResult<RecentDashboardResponse> {

    // r.node_config.self_client().metrics()

    let start = current_time_millis_i64();
    let recent = r.ds.transaction_store.query_recent_transactions(Some(10), is_test).await?;
    trace!("Dashboard query time elapsed: {:?}", current_time_millis_i64() - start);
    let mut recent_transactions = Vec::new();
    for tx in recent {
        let c = r.ds.observation.count_observation_edge(&tx.hash_or()).await;
        let brief_tx = explorer::brief_transaction(&tx, None, c.ok().map(|x| x as i64))?;
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