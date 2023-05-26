use std::sync::Arc;
use bitcoin::secp256k1::PublicKey;
use futures::{StreamExt, TryFutureExt, TryStreamExt};

use libp2p::Multiaddr;
use log::{error, info};
use tokio::runtime::Runtime;
use tokio::select;
use tokio::task::JoinHandle;
use redgold_schema::SafeOption;
use redgold_schema::structs::{ErrorInfo, PeerData};
use crate::api::RgHttpClient;
use crate::core::internal_message::{FutLoopPoll, PeerMessage, SendErrorInfo};
use crate::core::peer_rx_event_handler::rest_peer;

use crate::core::relay::Relay;
use crate::node_config::NodeConfig;
use crate::schema::structs::{Response, ResponseMetadata};
use crate::util;
use crate::util::{to_libp2p_peer_id, to_libp2p_peer_id_ser};
use crate::schema::json;

#[derive(Clone)]
pub struct PeerOutgoingEventHandler {
    // pub(crate) p2p_client: crate::api::p2p_io::rgnetwork::Client,
    relay: Relay,
    // rt: Arc<Runtime>
}
use futures::FutureExt;

impl PeerOutgoingEventHandler {

    //async fn send_message(&mut self) ->

    async fn handle_peer_message(relay: Relay, message: PeerMessage) -> Result<(), ErrorInfo> {
        let ser_msg = json(&message.request.clone())?;
        // info!("PeerOutgoingEventHandler received message {}", ser_msg);
        let peers = relay.ds.peer_store.all_peers().await?;
        let ser_msgp = json(&peers.clone())?;
        // info!("PeerOutgoingEventHandler query all peers {}", ser_msgp);

        if let Some(pk) = message.public_key {

            let vec = pk.serialize().to_vec();
            let peer = peers.iter()
                .find(|p| p.node_metadata.iter().find(|nmd|
                    nmd.public_key_bytes().map(|v| v == vec).unwrap_or(false)
                ).is_some());
            match peer {
                None => {
                    error!("Peer public key not found to send message to {}", hex::encode(vec));
                }
                Some(pd) => {
                    // TODO: Deal with this guy
                    tokio::spawn(Self::send_message_rest(message.clone(), pd.clone(), relay.node_config.clone()));
                }
            }

        } else {
            // let peers2 = peers.iter().map(|pd| pd.)
            // Relay::broadcast(relay.clone(), )
            // Change this to the same broadcast function as above^
            info!("Attempting broadcast to {}", &peers.len());
            for pd in &peers {
                // TODO: Deal with this guy too
                tokio::spawn(Self::send_message_rest(message.clone(), pd.clone(), relay.node_config.clone()));
            }
        }
        Ok(())
    }

    async fn run(&mut self) -> Result<(), ErrorInfo> {
        // let mut futs = crate::core::internal_message::FutLoopPoll::new();

        use futures::StreamExt;
        use crate::core::internal_message::RecvAsyncErrorInfo;

        let receiver = self.relay.peer_message_tx.receiver.clone();
        let relay = self.relay.clone();
        let err = receiver.into_stream().for_each_concurrent(200, |message| {
            async {
                Self::handle_peer_message(relay.clone(), message).await;
            }
        });
        err.await;
        Ok(())
    }

    //
    // async fn run(&mut self) -> Result<(), ErrorInfo> {
    //     let mut futs = crate::core::internal_message::FutLoopPoll::new();
    //
    //     use futures::StreamExt;
    //     use crate::core::internal_message::RecvAsyncErrorInfo;
    //     loop {
    //         // replace with mpsc
    //         select! {
    //             res = futs.futures.next() => {
    //                 crate::core::internal_message::FutLoopPoll::map_fut(res)?;
    //             }
    //             m = self.relay.peer_message_tx.receiver.recv_async_err() => {
    //                 let message: PeerMessage = m?;
    //                 let ser_msg = json(&message.request.clone())?;
    //                 // info!("PeerOutgoingEventHandler received message {}", ser_msg);
    //                 let peers = self.relay.ds.peer_store.all_peers().await?;
    //                 let ser_msgp = json(&peers.clone())?;
    //                 // info!("PeerOutgoingEventHandler query all peers {}", ser_msgp);
    //
    //                 if let Some(pk) = message.public_key {
    //
    //                     let vec = pk.serialize().to_vec();
    //                     let peer = peers.iter()
    //                         .find(|p| p.node_metadata.iter().find(|nmd|
    //                         nmd.public_key_bytes().map(|v| v == vec).unwrap_or(false)
    //                     ).is_some());
    //                     match peer {
    //                         None => {
    //                             error!("Peer public key not found to send message to {}", hex::encode(vec));
    //                         }
    //                         Some(pd) => {
    //                             futs.futures.push(
    //                                 tokio::spawn(Self::send_message_rest(message.clone(), pd.clone(), self.relay.node_config.clone()))
    //                             );
    //                         }
    //                     }
    //
    //                     } else {
    //                     info!("Attempting broadcast to {}", &peers.len());
    //                     for pd in &peers {
    //                         futs.futures.push(tokio::spawn(Self::send_message_rest(message.clone(), pd.clone(), self.relay.node_config.clone())));
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }
    //

    pub async fn send_message_rest(message: PeerMessage, pd: PeerData, nc: NodeConfig) -> Result<(), ErrorInfo> {

        let nmd = pd.node_metadata[0].clone();
        let res = rest_peer(
            nc, nmd.external_address.clone(), (nmd.port_offset.safe_get()? + 1), message.request.clone()
        ).await;
        match res {
            Ok(r) => {
                // info!("PeerOutgoingEventHandler sent message to {} and received response {}", nmd.external_address.clone(), json(&r.clone())?);
                if let Some(response_channel) = &message.response {
                    response_channel.send_err(r)?;
                } else {
                    info!("No response channel to respond with");
                }
            }
            Err(e)=> {
                log::error!("Error sending message to peer: {}", json(&e)?)
            }
        }
        Ok(())
    }


    // https://stackoverflow.com/questions/63347498/tokiospawn-borrowed-value-does-not-live-long-enough-argument-requires-tha
    pub fn new(relay: Relay
               // , rt: Arc<Runtime>
    ) -> JoinHandle<Result<(), ErrorInfo>> {
        let mut b = Self { relay
            // , rt: rt.clone()
        };
        return tokio::spawn(async move { b.run().await });
    }

    //
    // async fn send_message_libp2p(&mut self, message: &PeerMessage, pd: &PeerData) -> Result<Response, ErrorInfo> {
    //     let nmd = pd.node_metadata[0].clone();
    //     let pk = nmd.public_key;
    //     let address_str_external = format!("/ip4/{}/tcp/{}", nmd.external_address, nmd.port_offset.safe_get()?.to_string());
    //     let multi_addr: Multiaddr = address_str_external.parse().ok().map(|x| Ok(x)).unwrap_or(
    //         Err(ErrorInfo::error_info("Parse multiaddr fail")))?; // todo box dyn error
    //     let peer_id = to_libp2p_peer_id_ser(&pk);
    //     info!("Sending outgoing peer event to peer {:?} on address string {}", peer_id, address_str_external);
    //
    //     let res_response =
    //         self.p2p_client.dial_and_send(
    //             peer_id, multi_addr, message.request.clone()
    //         ).await;
    //     let res = match res_response {
    //         Ok(r) => {
    //             info!("Got response from p2pclient message after dialing in outgoing event handler");
    //             r
    //         }
    //         Err(e) => {
    //             error!("Error sending p2p message {:?}", e.clone());
    //             Response::from_error_info(e)
    //         }
    //     };
    //     Ok(res)
    // }

}
