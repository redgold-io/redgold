use std::sync::Arc;

use bitcoin::secp256k1::PublicKey;
use futures::{StreamExt, TryFutureExt, TryStreamExt};
use futures::FutureExt;
use libp2p::Multiaddr;
use log::{error, info};
use metrics::increment_counter;
use tokio::runtime::Runtime;
use tokio::select;
use tokio::task::JoinHandle;
use tracing::debug;

use redgold_schema::{error_info, json_or, SafeOption};
use redgold_schema::errors::EnhanceErrorInfo;
use redgold_schema::structs::{ErrorInfo, NetworkEnvironment, NodeMetadata, PeerData};

use crate::api::RgHttpClient;
use crate::core::internal_message::{PeerMessage, SendErrorInfo};
use crate::core::peer_rx_event_handler::rest_peer;
use crate::core::relay::Relay;
use crate::node_config::NodeConfig;
use crate::schema::json;
use crate::schema::structs::{Response, ResponseMetadata};
use crate::util;
use crate::util::{to_libp2p_peer_id, to_libp2p_peer_id_ser};

use redgold_schema::EasyJson;

#[derive(Clone)]
pub struct PeerOutgoingEventHandler {
    // pub(crate) p2p_client: crate::api::p2p_io::rgnetwork::Client,
    relay: Relay,
    // rt: Arc<Runtime>
}

impl PeerOutgoingEventHandler {

    async fn send_peer_message(relay: Relay, message: PeerMessage) -> Result<(), ErrorInfo> {
        increment_counter!("redgold.peer.send");
        let ser_msgp = json_or(&message.request.clone());
        // tracing::info!("PeerOutgoingEventHandler send message {}", ser_msgp);
        if let Some(pk) = &message.public_key {
            let res = relay.ds.peer_store.query_public_key_node(pk.clone()).await?
                .and_then(|pd| pd.latest_node_transaction)
                .and_then(|nt| nt.node_metadata().ok());
            // TODO if metadata known, check if udp is required
            if let Some(nmd) = res {
                Self::send_message_rest(message.clone(), nmd, relay.node_config.clone()).await?;
            } else {
                error!("Node metadata not found for peer public key to send message to {} contents: {}", pk.hex_or(), ser_msgp);
            }
        } else if let Some(nmd) = &message.node_metadata {
            debug!("PeerOutgoingEventHandler send message to node metadata {} with public key unregistered {}",
                nmd.json_or(),
                ser_msgp
            );
            Self::send_message_rest(message.clone(), nmd.clone(), relay.node_config.clone()).await?;
            // TODO: if node metadata in message then attempt to send there to unknown peer, falling back to other types
            // Do we also need dynamic node metadata here too for UDP?
        } else {
            let s = format!("Missing public key or node metadata in peer messsage, unable to process contents {}", ser_msgp);
            error!("{}", s);
            return Err(error_info(s));
        }
        Ok(())
    }

    async fn run(&mut self) -> Result<(), ErrorInfo> {

        use futures::StreamExt;
        use crate::core::internal_message::RecvAsyncErrorInfo;

        let receiver = self.relay.peer_message_tx.receiver.clone();
        let relay = self.relay.clone();
        let err = receiver.into_stream()
            .map(|x| Ok(x))
            .try_for_each_concurrent(200, |message| {
            async {
                Self::send_peer_message(relay.clone(), message).await
            }
        });
        err.await
    }

    pub async fn send_message_rest(mut message: PeerMessage, nmd: NodeMetadata, nc: NodeConfig) -> Result<(), ErrorInfo> {
        increment_counter!("redgold.peer.rest.send");


        let option = NetworkEnvironment::from_i32(nmd.network_environment);
        let peer_env = option
            .safe_get_msg("Missing network environment in node metadata on attempt to send peer message")?;
        if peer_env != &nc.network {
            return Err(error_info(format!("\
            Attempted to send message to peer {} with network {} while this node is on network {} contents: {}",
                                          nmd.long_identifier(), peer_env.to_std_string(), nc.network.to_std_string(),
                                          json_or(&message.request.clone())
            )));
        }
        let port = nmd.port_or(nc.network) + 1;
        let res = rest_peer(
            nc, nmd.external_address.clone(), nmd.port_offset
                .safe_get_msg("Missing port offset in node metadata on attempt to send peer message")? + 1, &mut message.request
        ).await;
        match res {
            Ok(r) => {
                // debug!("Send message peer response: {}", json_or(&r));
                if let Some(response_channel) = &message.response {
                    response_channel.send_err(r).add("Error sending message back on response channel")?;
                }
            }
            Err(e)=> {
                increment_counter!("redgold.peer.rest.send.error");
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
