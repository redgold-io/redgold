use std::convert::identity;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures::channel::mpsc::Receiver;
use futures::prelude::*;
use itertools::Itertools;
// use libp2p::{Multiaddr, PeerId};
// use libp2p::request_response::ResponseChannel;
use log::{debug, error, info, trace};
use metrics::{counter, histogram};
// use svg::Node;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

use redgold_schema::{error_info, RgResult, SafeOption, structs};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::{AboutNodeRequest, AboutNodeResponse, ErrorInfo, GetPartiesInfoResponse, GetPeersInfoRequest, GetPeersInfoResponse, Hash, PublicKey, QueryObservationProofResponse, RecentDiscoveryTransactionsResponse, Request, ResolveCodeResponse, SubmitTransactionRequest, TransactionEntry, UtxoId, UtxoValidResponse};

use crate::api::about;
use crate::core::discover::peer_discovery::DiscoveryMessage;
// use crate::api::p2p_io::rgnetwork::{Client, Event, PeerResponse};
use crate::core::internal_message::{new_channel, PeerMessage, RecvAsyncErrorInfo, SendErrorInfo, TransactionMessage};
use crate::core::relay::Relay;
use redgold_data::data_store::DataStore;
use redgold_keys::request_support::{RequestSupport, ResponseSupport};
use redgold_schema::helpers::easy_json::json_or;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use crate::data::download::process_download_request;
use crate::multiparty_gg20::initiate_mp::{initiate_mp_keygen, initiate_mp_keygen_follower, initiate_mp_keysign, initiate_mp_keysign_follower};
use crate::node_config::NodeConfig;
use redgold_schema::helpers::easy_json::json;
use crate::schema::response_metadata;
use crate::schema::structs::{Response, ResponseMetadata};
use crate::util::keys::ToPublicKeyFromLib;
use redgold_schema::util::lang_util::{SameResult, WithMaxLengthString};
use crate::api::faucet::faucet_request;
// use crate::multiparty_gg20::watcher::DepositWatcher;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::proto_serde::ProtoSerde;
use crate::observability::metrics_help::WithMetrics;
use redgold_schema::structs::BatchTransactionResolveResponse;
use redgold_schema::util::timers::PerfTimer;
use crate::api::hash_query::get_address_info_public_key;

pub struct PeerRxEventHandler {
    relay: Relay,
    // rt: Arc<Runtime>
}

impl PeerRxEventHandler {

    pub async fn handle_incoming_message(
        relay: Relay, pm: PeerMessage
        // , rt: Arc<Runtime>
    ) -> Result<(), ErrorInfo> {

        let pt = PerfTimer::new();
        counter!("redgold.peer.message.received").increment(1);

        // This is important for some requests but not others, use in case by case basis
        let verified = pm.request.verify_auth().add("Incoming request authorization failure in peer rx event handler");

        // Check if we know the peer, if not, attempt discovery
        if let Some(pk) = pm.request.clone().proof.clone().and_then(|r| r.public_key) {
            let known = relay.ds.peer_store.query_public_key_node(&pk).await?.is_some();
            if known {
                relay.ds.peer_store.update_last_seen(&pk).await.ok();
            } else {
                if let Some(nmd) = &pm.request.node_metadata {
                    info!("Attempting immediate discovery on peer {}", pk.short_id());
                    relay.discovery.sender.send_rg_err(
                        DiscoveryMessage::new(nmd.clone(), pm.dynamic_node_metadata.clone())
                    ).log_error().ok();
                }
            }
        }


        // Handle the request
        // tracing::debug!("Peer Rx Event Handler received request {}", json(&pm.request)?);
        let response = Self::request_response(relay.clone(), pm.request.clone(), verified.clone(), pm.socket_addr).await
            .map_err(|e| Response::from_error_info(e)).combine()
            .with_metadata(relay.node_metadata().await?)
            .with_auth(&relay.node_config.keypair())
            .verify_auth(Some(&relay.node_config.public_key())).expect("immediate verify");

        let response_time_secs = pt.seconds();

        if pm.request.recent_transactions_request.is_some() {
            // info!("Recent transactions request took {} seconds", response_time_secs);
            histogram!("redgold.peer.message.recent_transactions_request");
        }

        if let Some(c) = pm.response {
            // let _ser = response.clone().json_or();
            // let _peer = verified.clone().map(|p| p.short_id()).unwrap_or("unknown".to_string());
            // debug!("Sending response to peer {} contents {}", peer, ser);
            c.send_rg_err(response)
                .add("Send message to response channel failed in handle incoming message")
                .with_detail("response_time_secs", response_time_secs.to_string())
                .with_detail("request", pm.request.json_or().with_max_length(1000))
                .log_error().ok();
        }

        Ok(())

    }


    pub async fn request_response(
        relay: Relay,
        request: Request,
        verified: RgResult<PublicKey>,
        origin_ip: Option<SocketAddr>
    ) -> RgResult<Response> {


        if let Some(addr) = origin_ip.as_ref() {
            let ip = addr.ip().to_string();
            let labels = [("ip".to_string(), ip)];
            counter!("redgold_request_response_ip", &labels).increment(1)
        }

        // TODO: Rate limiting here

        // TODO: add a uuid here

        if let Ok(npk) = verified.as_ref() {
            let npk_hex = npk.hex();
            let labels = [("public_key".to_string(), npk_hex)];
            counter!("redgold_request_response_pk", &labels).increment(1);
            if !relay.is_seed(npk).await {
                if let Some(nmd) = request.node_metadata.as_ref() {
                    let opt_reward = relay.ds.peer_store.query_nodes_peer_node_info(&npk).await.ok()
                        .and_then(|x| x)
                        .and_then(|x| x.latest_peer_transaction)
                        .and_then(|x| x.peer_data().ok())
                        .and_then(|x| x.reward_address);
                    let addr = opt_reward.or(npk.address().ok());
                    if let Some(a) = addr {
                        let h = a.hex();
                        let labels = [("address".to_string(), h)];
                        counter!("redgold_request_reward", &labels).increment(1)
                    }
                }
            }
        }

        let mut response = Response::empty_success();

        let auth_required = request.auth_required();

        if let Some(r) = &request.get_active_party_key_request {
            response.get_active_party_key_response = relay.active_party_key().await;
        }

        if let Some(fr) = &request.faucet_request {
            response.faucet_response = Some(faucet_request(fr, &relay, request.origin.as_ref()).await.log_error().with_err_count("redgold.faucet.error")?);
        }

        if let Some(pk) = &request.get_address_info_public_key_request {
            response.get_address_info_public_key_response = Some(get_address_info_public_key(&relay, pk, None, None)
                .await
                .log_error()
                .with_err_count("redgold_get_address_info_public_key_error")?);
        }

        if let Some(_) = &request.get_parties_info_request {
            // let mut get_parties_info_response = GetPartiesInfoResponse::default();
            // let mut vec = vec![];
            // if let Some(c) = DepositWatcher::get_deposit_config(&relay.ds).await? {
            //     for a in &c.deposit_allocations {
            //         if let Ok(pi) = a.party_info() {
            //             vec.push(pi)
            //         }
            //     }
            // }
            // get_parties_info_response.party_info = vec;
            // response.get_parties_info_response = Some(get_parties_info_response);
            Err(error_info("Not implemented"))?;
        }

        if let Some(r) = &request.lookup_transaction_request {
            let opt = relay.lookup_transaction(r).await?;
            response.lookup_transaction_response = opt;
        }

        if let Some(_) = &request.genesis_request {
            response.genesis_response = relay.ds.config_store.get_genesis().await?.clone();
        }

        if let Some(r) = &request.recent_transactions_request {
            let from_mempool = relay.mempool_entries.clone().iter().map(|t| t.hash_or()).collect_vec();
            let in_process = relay.transaction_channels.iter().map(|c| c.transaction_hash.clone()).collect_vec();
            let accepted = relay.ds.transaction_store
                .recent_transaction_hashes(r.limit, r.min_time).await?;
            let mut hashes = vec![];
            hashes.extend(from_mempool);
            hashes.extend(in_process);
            hashes.extend(accepted);
            let res = RecentDiscoveryTransactionsResponse {
                transaction_hashes: hashes
            };
            response.recent_discovery_transactions_response = Some(res);
        }

        if let Some(r) = &request.utxo_valid_request {
            // TODO: Check transaction edge for utxo considered invalid.
            let rr: &UtxoId = r;
            let res = relay.ds.utxo.utxo_id_valid(r).await?;
            let mut u = UtxoValidResponse {
                valid: None,
                child_transaction: None,
                child_transaction_input: None,
            };
            if res {
                u.valid = Some(true);
            } else {
                let child = relay.ds.utxo.utxo_child(rr).await?;
                if let Some((child_hash, child_idx)) = child {
                    if let Some((tx, e)) = relay.ds.transaction_store.query_maybe_transaction(&child_hash).await? {
                        if e.is_none() {
                            u.valid = Some(false);
                            u.child_transaction = Some(tx);
                            u.child_transaction_input = Some(child_idx);
                        }
                    }

                }
            }
            response.utxo_valid_response = Some(u);
        }

        if let Some(addr) = &request.resolve_code_request {
            response.resolve_code_response = Some(relay.ds.resolve_code(addr).await?);
        }

        if let Some(r) = &request.get_contract_state_marker_request {
            if let (Some(a), s) = (&r.address, r.selector.as_ref()) {
                let res = relay.ds.state.query_recent_state(a, s, Some(1)).await?;
                if let Some(csm) = res.get(0) {
                    response.get_contract_state_marker_response = Some(csm.clone());
                }
            } else {
                return Err(error_info("Missing address or utxo id"));
            }
        }

        // TODO: Check for auth info and use for rate limiting
        // oooh need a request id, 2 of them
        // No auth required requests first
        if let Some(r) = request.hash_search_request {
            response.hash_search_response =
                Some(crate::api::hash_query::hash_query(relay.clone(), r.search_string, None, None).await?);
        }
        // TODO: implement this, but first question is why isn't the observation handler accepting them properly?
        if let Some(r) = request.query_observation_proof_request {
            let h = r.hash.safe_get()?;
            let proofs = relay.ds.observation.select_observation_edge(h).await?;
            let mut query_observation_proof_response = QueryObservationProofResponse::default();
            query_observation_proof_response.observation_proof = proofs;
            response.query_observation_proof_response = Some(query_observation_proof_response);
        }

        if let Some(s) = request.submit_transaction_request {
            // debug!("Received submit transaction request, sending to relay");
            response.submit_transaction_response = Some(
                relay.submit_transaction(s, verified.clone().ok(), origin_ip.map(|s| s.to_string())).await?);
        } // else
        // if let some(f) = request.fau
        if let Some(_) = request.get_peers_info_request {
            let mut get_peers_info_response = GetPeersInfoResponse::default();
            let vec = relay.ds.peer_store.all_peers_info().await?;
            let self_info = relay.peer_node_info().await?;
            get_peers_info_response.peer_info = vec;
            get_peers_info_response.self_info = Some(self_info);
            response.get_peers_info_response = Some(get_peers_info_response);
            // response.get_peers_info_response = Some(relay.get_peers_info(r).await?);
        }

        if let Some(t) = request.gossip_transaction_request {
            if let Some(t) = t.transaction {
                counter!("redgold_gossip_transaction_received").increment(1);
                trace!("Received gossip transaction request for {}", &t.hash_or().hex());
                relay.submit_transaction(SubmitTransactionRequest {
                    transaction: Some(t),
                    sync_query_response: false,
                },verified.clone().ok(), origin_ip.map(|s| s.to_string())).await?;
            }
        }
        // TODO: Merge this into gossip transaction when process tx handles obs.
        if let Some(o) = request.gossip_observation_request {
            relay.observation.sender.send_rg_err(o.observation.unwrap())?;
        }

        if let Some(download_request) = request.download_request {
            // info!("Received download request");
            let result = process_download_request(&relay, download_request, verified.as_ref().ok()).await?;
            response.download_response = Some(result);
        }

        if let Some(r) = request.about_node_request {
            info!("Received about request");
            response.about_node_response = Some(about::handle_about_node(r, relay.clone()).await?);
        }

        if let Some(r) = request.batch_transaction_resolve_request {
            let h: Vec<Hash> = r.hashes.clone();
            let mut resp = vec![];
            for hh in h.iter() {
                // TODO: Remove this after historical obs tx also in tx db.
                if r.is_observation.unwrap_or(false) {
                    if let Some(tx) = relay.ds.observation.query_observation_entry(hh).await? {
                        resp.push(tx)
                    }
                } else {
                    let res = relay.ds.transaction_store.query_accepted_tx(hh).await?;
                    if let Some(tx) = res {
                        let txe = TransactionEntry {
                            time: tx.time()?.clone() as u64,
                            transaction: Some(tx),
                        };
                        resp.push(txe)
                    }
                }
            }
            let res = BatchTransactionResolveResponse {
                transactions: resp
            };
            response.batch_transaction_resolve_response = Some(res);
        }

        // Verified requests only below here
        if auth_required {
            match verified {
                Ok(pk) => {
                    if let Some(r) = &request.initiate_keygen {
                        // TODO Track future with loop poll pattern
                        // oh wait can we remove this spawn entirely?
                        // info!("Received MP request on peer rx: {}", json_or(&r));
                        let rel2 = relay.clone();
                        // TODO: Can we remove this spawn now that we have the spawn inside the initiate from main?
                        // tokio::spawn(async move {
                        let result1 = initiate_mp_keygen_follower(
                            rel2.clone(), r.clone(), &pk).await;
                        let mp_response: String = result1.clone()
                            .map(|x| json_or(&x)).map_err(|x| json_or(&x)).combine();
                        // info!("Multiparty response from follower: {}", mp_response);

                        response.initiate_keygen_response = Some(result1?);

                        // });
                    }
                    if let Some(k) = &request.initiate_signing {
                        let rel2 = relay.clone();
                        // info!("Received MP signing request on peer rx: {}", json_or(&k.clone()));
                        // TODO: Can we remove this spawn now that we have the spawn inside the initiate from main?
                        // tokio::spawn(async move {
                        let result1 = initiate_mp_keysign_follower(rel2.clone(), k.clone(), &pk).await;
                        let mp_response: String = result1.clone()
                            .map(|x| json_or(&x)).map_err(|x| json_or(&x)).combine();
                        // info!("Multiparty signing response from follower: {}", mp_response);
                        response.initiate_signing_response = Some(result1?);
                        // });
                    }
                }
                Err(e) => { return Err(e).add("Unable to process request, authorization required and failed").log_error(); }
            }
        }

        Ok(response)
    }

    async fn run(&mut self) -> Result<(), ErrorInfo> {
        let receiver = self.relay.peer_message_rx.receiver.clone();
        let relay = self.relay.clone();
        receiver.into_stream().map(|r| Ok(r)).try_for_each_concurrent(10, |pm| {
            // info!("Received peer message");
            Self::handle_incoming_message(relay.clone(), pm)
        }).await
    }


    // https://stackoverflow.com/questions/63347498/tokiospawn-borrowed-value-does-not-live-long-enough-argument-requires-tha
    pub fn new(relay: Relay,
               // arc: Arc<Runtime>
    ) -> JoinHandle<Result<(), ErrorInfo>> {
        let mut b = Self {
            relay,
            // rt: arc.clone()
        };
        tokio::spawn(async move { b.run().await })
    }
}







//
// pub async fn libp2p_handle_inbound2(
//     relay: Relay, request: Request, peer: PeerId, remote_address: Multiaddr, rt: Arc<Runtime>, mut p2p_client: Client
// ) -> Result<Response, ErrorInfo> {
//
//     info!("Received peer inbound request: {} from {:?}", serde_json::to_string(&request.clone()).unwrap(), peer.clone());
//
//     // let peer_lookup = relay.ds.peer_store.multihash_lookup(peer.to_bytes()).await?;
//     let peers = relay.ds.peer_store.all_peers().await?;
//     let mh = peer.to_bytes();
//     let known_peer = peers.iter().find(|p|
//         p.node_metadata.iter().find(|nmd| nmd.multi_hash == mh).is_some()
//     );
//     info!("Is peer known?: {:?}", serde_json::to_string(&known_peer.clone()).unwrap());
//
//     let response = PeerRxEventHandler::request_response(
//         relay.clone(), request.clone(),
//         // rt.clone()
//     ).await?;
//     //
//     // if known_peer.is_none() {
//     //     let client = p2p_client.clone();
//     //     let relay = relay.clone();
//     //     // info!("Requesting peer info on runtime");
//     //     rt.spawn(async move {
//     //         libp2p_request_peer_info(client, peer, remote_address, relay).await;
//     //     });
//     // }
//     Ok(response)
//
// }
//
//

//
// async fn libp2p_handle_about_response(
//     response: Result<Response, ErrorInfo>, peer_id: PeerId, addr: Multiaddr, relay: Relay
// ) -> Result<(), ErrorInfo> {
//     let res = response?.about_node_response.safe_get()?.latest_metadata.safe_get()?.clone();
//     // TODO: Validate transaction here
//     // relay.ds.peer_store.add_peer(&res, 0f64).await?;
//     Ok(())
// }
//
// async fn libp2p_request_peer_info(mut p2p_client: Client, peer_id: PeerId, addr: Multiaddr, relay: Relay) {
//     counter!("redgold.p2p.request_peer_info").increment(1);
//     info!("Requesting peer info for {:?} on addr {:?}", peer_id, addr.clone());
//     let mut r = Request::default();
//     r.about_node_request = Some(AboutNodeRequest{
//         verbose: false
//     });
//     let res = libp2p_handle_about_response(
//         p2p_client.dial_and_send(peer_id, addr.clone(), r).await, peer_id, addr.clone(), relay
//     ).await;
//     match res {
//         Ok(o) => {
//         }
//         Err(e) => {
//             error!("Error requesting peer info {}", serde_json::to_string(&e).unwrap());
//         }
//     }
//
// }
//
// pub async fn libp2phandle_inbound(relay: Relay, e: Event, mut p2p_client: Client, rt: Arc<Runtime>) -> Result<(), ErrorInfo> {
//     let Event::InboundRequest {
//         request,
//         peer,
//         channel,
//         remote_address,
//     } = e;
//     let response = libp2p_handle_inbound2(
//         relay, request.clone(), peer.clone(), remote_address.clone(), rt.clone(), p2p_client.clone()
//     ).await.map_err(|e| Response::from_error_info(e)).combine();
//     p2p_client.respond(response.clone(), channel).await;
//     Ok(())
//
// }
