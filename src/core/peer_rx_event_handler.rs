use crate::api::p2p_io::rgnetwork::{Client, Event, PeerResponse};
use crate::core::relay::{MultipartyRequestResponse, Relay};
use crate::data::data_store::DataStore;
use crate::data::download::process_download_request;
use crate::schema::response_metadata;
use crate::schema::structs::{Response, ResponseMetadata};
use futures::channel::mpsc::Receiver;
use futures::prelude::*;
use log::{error, info};
use std::convert::identity;
use std::net::SocketAddr;
use std::sync::Arc;
use libp2p::{Multiaddr, PeerId};
use libp2p::request_response::ResponseChannel;
use metrics::increment_counter;
use svg::Node;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use redgold_schema::structs::{AboutNodeRequest, AboutNodeResponse, ErrorInfo, MultipartyThresholdResponse, Request};
use redgold_schema::{json_or, SafeBytesAccess, SafeOption, structs, WithMetadataHashable};
use crate::core::internal_message::{FutLoopPoll, new_channel, PeerMessage, RecvAsyncErrorInfo, SendErrorInfo, TransactionMessage};
use crate::multiparty::initiate_mp::{initiate_mp_keygen, initiate_mp_keygen_follower, initiate_mp_keysign, initiate_mp_keysign_follower};
use crate::node_config::NodeConfig;
use crate::util::lang_util::SameResult;
use crate::schema::json;
use crate::util::keys::ToPublicKeyFromLib;

pub async fn rest_peer(nc: NodeConfig, ip: String, port: i64, mut request: Request) -> Result<Response, ErrorInfo> {
    let client = crate::api::HTTPClient::new(ip, port as u16);
    client.proto_post(
        request
            .with_auth(&nc.wallet().active_keypair())
            .with_metadata(nc.node_metadata())
        , "request_proto".to_string()).await
}

pub struct PeerRxEventHandler {
    relay: Relay,
    rt: Arc<Runtime>
}

use crate::util::logging::Loggable;

impl PeerRxEventHandler {

    pub async fn request_response_rest(
        relay: Relay, pm: PeerMessage, rt: Arc<Runtime>
    ) -> Result<(), ErrorInfo> {
        // pm.request.verify_auth()?;
        if let Some(p) = pm.public_key {
            let struct_pk = structs::PublicKey::from_bytes(p.serialize().to_vec());
            relay.ds.peer_store.update_last_seen(struct_pk).await;;
        }

        // info!("Peer Rx Event Handler received request {}", json(&pm.request)?);
        let response = Self::request_response(relay.clone(), pm.request.clone(), rt.clone()).await
            .map_err(|e| Response::from_error_info(e)).combine();
        if let Some(c) = pm.response {
            // info!("Sending response to channel");
            c.send_err(response)?;
        } else {
            // info!("No response channel");
        }

        let peers = relay.ds.peer_store.all_peers().await?;

        if let Some(prf) = pm.request.proof.clone() {
            if let Some(pk) = prf.public_key {

                let known_peer = peers.iter().find(|p|
                    p.node_metadata.iter().find(|nmd|
                        match (nmd.public_key_bytes().ok(), pk.bytes.safe_bytes().ok()) {
                            (Some(pk1), Some(pk2)) => pk1 == pk2,
                            _ => false
                        }).is_some()
                );
                // info!("Is peer known?: {:?}", json(&known_peer.clone())?);

                if known_peer.is_none() {
                    if let Some(nmd) = pm.request.node_metadata {

                        let mut request = relay.node_config.request().about();

                        let relay = relay.clone();
                        info!("Requesting peer info on runtime");
                        rt.spawn(async move {
                            let response = rest_peer(
                                relay.node_config.clone(), nmd.external_address.clone(),
                                (nmd.port_or(relay.node_config.network.clone()) as i64) + 1,
                                request.clone()
                            ).await;
                            Self::handle_about_peer_response(relay.clone(), response).await
                        });
                    }

                }
            }


        } else {
            info!("No proof on incoming request, unknown peer");
        }
        Ok(())

    }


    async fn handle_about_peer_response(
        relay: Relay, response: Result<Response, ErrorInfo>
    ) -> Result<(), ErrorInfo> {
        let res = response?.about_node_response.safe_get()?.latest_metadata.safe_get()?.clone();
        // TODO: Validate transaction here
        info!("Added new peer: {}", json(&res)?);
        relay.ds.peer_store.add_peer(&res, 0f64).await?;
        Ok(())
    }

    pub async fn request_response(relay: Relay, request: Request, arc: Arc<Runtime>) -> Result<Response, ErrorInfo> {

        let mut response = Response::empty_success();
        if let Some(t) = request.gossip_transaction_request {
            // info!("Received gossip transaction request");
            relay
                .transaction
                .sender
                .send(TransactionMessage {
                    transaction: t.transaction.unwrap(),
                    response_channel: None,
                })
                .expect("Transaction send failure");
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
            let result = DataStore::map_err(process_download_request(&relay, download_request))?;
            response.download_response = Some(result);
        }

        if let Some(abr) = request.about_node_request {
            // info!("Received about request");
            let mut abr = AboutNodeResponse::empty();
            abr.latest_metadata = Some(relay.node_config.peer_data_tx());
            response.about_node_response = Some(abr);
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
            let mut res_inner = MultipartyThresholdResponse::empty();

            if let Some(k) = r.initiate_keygen {
                // TODO Track future with loop poll pattern
                // oh wait can we remove this spawn entirely?
                info!("Received MP request on peer rx: {}", json_or(&k));
                let rel2 = relay.clone();
                // TODO: Can we remove this spawn now that we have the spawn inside the initiate from main?
                arc.spawn(async move {
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
                arc.spawn(async move {
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

    async fn run(&mut self) -> Result<(), ErrorInfo> {

        // Wait a minute if we're polling these futures do we even need a spawn here?
        let mut fut = FutLoopPoll::new();

        let receiver = self.relay.peer_message_rx.receiver.clone();
        fut.run(receiver, |pm| {
            increment_counter!("redgold.peer.message.received");
            // info!("Peer rx event handler received message");
            self.rt.spawn({
                Self::request_response_rest(self.relay.clone(), pm.clone(), self.rt.clone())
            })
        }).await
    }
    // https://stackoverflow.com/questions/63347498/tokiospawn-borrowed-value-does-not-live-long-enough-argument-requires-tha
    pub fn new(relay: Relay, arc: Arc<Runtime>) -> JoinHandle<Result<(), ErrorInfo>> {
        let mut b = Self {
            relay,
            rt: arc.clone()
        };
        arc.spawn(async move { b.run().await })
    }
}








pub async fn libp2p_handle_inbound2(
    relay: Relay, request: Request, peer: PeerId, remote_address: Multiaddr, rt: Arc<Runtime>, mut p2p_client: Client
) -> Result<Response, ErrorInfo> {

    info!("Received peer inbound request: {} from {:?}", serde_json::to_string(&request.clone()).unwrap(), peer.clone());

    // let peer_lookup = relay.ds.peer_store.multihash_lookup(peer.to_bytes()).await?;
    let peers = relay.ds.peer_store.all_peers().await?;
    let mh = peer.to_bytes();
    let known_peer = peers.iter().find(|p|
        p.node_metadata.iter().find(|nmd| nmd.multi_hash == mh).is_some()
    );
    info!("Is peer known?: {:?}", serde_json::to_string(&known_peer.clone()).unwrap());

    let response = PeerRxEventHandler::request_response(
        relay.clone(), request.clone(), rt.clone()).await?;

    if known_peer.is_none() {
        let client = p2p_client.clone();
        let relay = relay.clone();
        // info!("Requesting peer info on runtime");
        rt.spawn(async move {
            libp2p_request_peer_info(client, peer, remote_address, relay).await;
        });
    }
    Ok(response)

}




async fn libp2p_handle_about_response(
    response: Result<Response, ErrorInfo>, peer_id: PeerId, addr: Multiaddr, relay: Relay
) -> Result<(), ErrorInfo> {
    let res = response?.about_node_response.safe_get()?.latest_metadata.safe_get()?.clone();
    // TODO: Validate transaction here
    // relay.ds.peer_store.add_peer(&res, 0f64).await?;
    Ok(())
}

async fn libp2p_request_peer_info(mut p2p_client: Client, peer_id: PeerId, addr: Multiaddr, relay: Relay) {
    increment_counter!("redgold.p2p.request_peer_info");
    info!("Requesting peer info for {:?} on addr {:?}", peer_id, addr.clone());
    let mut r = Request::default();
    r.about_node_request = Some(AboutNodeRequest{
        verbose: false
    });
    let res = libp2p_handle_about_response(
        p2p_client.dial_and_send(peer_id, addr.clone(), r).await, peer_id, addr.clone(), relay
    ).await;
    match res {
        Ok(o) => {
        }
        Err(e) => {
            error!("Error requesting peer info {}", serde_json::to_string(&e).unwrap());
        }
    }

}

pub async fn libp2phandle_inbound(relay: Relay, e: Event, mut p2p_client: Client, rt: Arc<Runtime>) -> Result<(), ErrorInfo> {
    let Event::InboundRequest {
        request,
        peer,
        channel,
        remote_address,
    } = e;
    let response = libp2p_handle_inbound2(
        relay, request.clone(), peer.clone(), remote_address.clone(), rt.clone(), p2p_client.clone()
    ).await.map_err(|e| Response::from_error_info(e)).combine();
    p2p_client.respond(response.clone(), channel).await;
    Ok(())

}
