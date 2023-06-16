use std::convert::identity;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures::channel::mpsc::Receiver;
use futures::prelude::*;
use libp2p::{Multiaddr, PeerId};
use libp2p::request_response::ResponseChannel;
use log::{debug, error, info};
use metrics::increment_counter;
use svg::Node;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

use redgold_schema::{json_or, RgResult, SafeBytesAccess, SafeOption, structs, WithMetadataHashable};
use redgold_schema::EasyJson;
use redgold_schema::errors::EnhanceErrorInfo;
use redgold_schema::structs::{AboutNodeRequest, AboutNodeResponse, ErrorInfo, GetPeersInfoRequest, GetPeersInfoResponse, MultipartyThresholdResponse, PublicKey, QueryObservationProofResponse, Request, SubmitTransactionRequest};

use crate::api::about;
use crate::core::discovery::DiscoveryMessage;
// use crate::api::p2p_io::rgnetwork::{Client, Event, PeerResponse};
use crate::core::internal_message::{new_channel, PeerMessage, RecvAsyncErrorInfo, SendErrorInfo, TransactionMessage};
use crate::core::relay::{MultipartyRequestResponse, Relay};
use crate::data::data_store::DataStore;
use crate::data::download::process_download_request;
use crate::multiparty::initiate_mp::{initiate_mp_keygen, initiate_mp_keygen_follower, initiate_mp_keysign, initiate_mp_keysign_follower};
use crate::node_config::NodeConfig;
use crate::schema::json;
use crate::schema::response_metadata;
use crate::schema::structs::{Response, ResponseMetadata};
use crate::util::keys::ToPublicKeyFromLib;
use crate::util::lang_util::SameResult;
use crate::util::logging::Loggable;

pub async fn rest_peer(nc: NodeConfig, ip: String, port: i64, request: &mut Request) -> Result<Response, ErrorInfo> {
    let client = crate::api::RgHttpClient::new(ip, port as u16);
    client.proto_post_request(request, Some(nc)).await
}

pub struct PeerRxEventHandler {
    relay: Relay,
    // rt: Arc<Runtime>
}

impl PeerRxEventHandler {

    pub async fn handle_incoming_message(
        relay: Relay, pm: PeerMessage
        // , rt: Arc<Runtime>
    ) -> Result<(), ErrorInfo> {
        increment_counter!("redgold.peer.message.received");

        // This is important for some requests but not others, use in case by case basis
        let verified = pm.request.verify_auth();

        // Check if we know the peer, if not, attempt discovery
        if let Some(pk) = pm.request.clone().proof.clone().and_then(|r| r.public_key) {
            let known = relay.ds.peer_store.query_public_key_node(pk.clone()).await?.is_some();
            if known {
                relay.ds.peer_store.update_last_seen(pk).await.ok();
            } else {
                if let Some(nmd) = &pm.request.node_metadata {
                    info!("Attempting immediate discovery on peer {}", pk.short_id());
                    relay.discovery.sender.send_err(
                        DiscoveryMessage::new(nmd.clone(), pm.dynamic_node_metadata.clone())
                    ).log_error().ok();
                }
            }
        }

        // Handle the request

        // tracing::debug!("Peer Rx Event Handler received request {}", json(&pm.request)?);
        let mut response = Self::request_response(relay.clone(), pm.request.clone(), verified.clone()).await
            .map_err(|e| Response::from_error_info(e)).combine();
        response.with_metadata(relay.node_config.node_metadata());
        response.with_auth(&relay.node_config.internal_mnemonic().active_keypair());
        if let Some(c) = pm.response {
            let ser = response.clone().json_or();
            let peer = verified.clone().map(|p| p.short_id()).unwrap_or("unknown".to_string());
            // debug!("Sending response to peer {} contents {}", peer, ser);
            c.send_err(response).add("Send message to response channel failed in handle incoming message")?;
        }

        Ok(())

    }


    pub async fn request_response(relay: Relay, request: Request, verified: RgResult<PublicKey>
                                  // , arc: Arc<Runtime>
    ) -> Result<Response, ErrorInfo> {


        // TODO: Rate limiting here

        // TODO: add a uuid here
        let mut response = Response::empty_success();

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
            response.submit_transaction_response = Some(relay.submit_transaction(s).await?);
        } // else
        // if let some(f) = request.fau
        if let Some(_) = request.get_peers_info_request {
            let mut get_peers_info_response = GetPeersInfoResponse::default();
            let vec = relay.ds.peer_store.peer_node_info().await?;
            get_peers_info_response.peer_info = vec;
            response.get_peers_info_response = Some(get_peers_info_response);
            // response.get_peers_info_response = Some(relay.get_peers_info(r).await?);
        }

        if let Some(t) = request.gossip_transaction_request {
            if let Some(t) = t.transaction {
                info!("Received gossip transaction request for {}", &t.hash_or().hex());
                relay.submit_transaction(SubmitTransactionRequest {
                    transaction: Some(t),
                    sync_query_response: false,
                }).await?;
            }
        }
        if let Some(o) = request.gossip_observation_request {
            // info!("Received gossip observation request");
            relay
                .observation
                .sender
                .send(o.observation.unwrap())
                .expect("gossip send failure");
        }

        if let Some(download_request) = request.download_request {
            // info!("Received download request");
            let result = DataStore::map_err(process_download_request(&relay, download_request).await)?;
            response.download_response = Some(result);
        }

        if let Some(r) = request.about_node_request {
            info!("Received about request");
            response.about_node_response = Some(about::handle_about_node(r, relay.clone()).await?);
        }

        if let Some(r) = request.multiparty_threshold_request {
            info!("Received multiparty threshold request");
            // let (sender, rec) =
            //     flume::bounded::<MultipartyThresholdResponse>(1);
            // let result = relay.multiparty.sender.send(MultipartyRequestResponse{
            //     request: Some(r),
            //     response: None,
            //     sender: Some(sender),
            //     origin: request.node_metadata.clone(),
            //     internal_subscribe: None,
            // }).expect("Multiparty send failure");
            // let result = rec.recv_async_err().await?;
            // response.multiparty_threshold_response = Some(result);
            let _res_inner = MultipartyThresholdResponse::empty();

            if let Some(k) = r.initiate_keygen {
                // TODO Track future with loop poll pattern
                // oh wait can we remove this spawn entirely?
                info!("Received MP request on peer rx: {}", json_or(&k));
                let rel2 = relay.clone();
                // TODO: Can we remove this spawn now that we have the spawn inside the initiate from main?
                tokio::spawn(async move {
                    let result1 = initiate_mp_keygen_follower(
                        rel2.clone(), k).await;
                    let mp_response: String = result1.clone()
                        .map(|x| json_or(&x)).map_err(|x| json_or(&x)).combine();
                    info!("Multiparty response from follower: {}", mp_response);
                });
                // });
            }
            if let Some(k) = r.initiate_signing {
                let rel2 = relay.clone();
                info!("Received MP signing request on peer rx: {}", json_or(&k.clone()));
                // TODO: Can we remove this spawn now that we have the spawn inside the initiate from main?
                tokio::spawn(async move {
                    let result1 = initiate_mp_keysign_follower(rel2.clone(), k).await;
                    let mp_response: String = result1.clone()
                        .map(|x| json_or(&x)).map_err(|x| json_or(&x)).combine();
                    info!("Multiparty signing response from follower: {}", mp_response);
                });
            }

        }
        // info!(
        //                 "Preparing response to peer RX event handler: {}",
        //                 serde_json::to_string(&response.clone()).unwrap_or("json fail".into())
        //             );
        Ok(response)
    }
    //
    // async fn run(&mut self) -> Result<(), ErrorInfo> {
    //
    //     // Wait a minute if we're polling these futures do we even need a spawn here?
    //     let mut fut = FutLoopPoll::new();
    //
    //     let receiver = self.relay.peer_message_rx.receiver.clone();
    //     fut.run(receiver, |pm| {
    //         increment_counter!("redgold.peer.message.received");
    //         // info!("Peer rx event handler received message");
    //         tokio::spawn({
    //             Self::request_response_rest(self.relay.clone(), pm.clone(),
    //                                         // self.rt.clone()
    //             )
    //         })
    //     }).await
    // }
    //

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
//     increment_counter!("redgold.p2p.request_peer_info");
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
