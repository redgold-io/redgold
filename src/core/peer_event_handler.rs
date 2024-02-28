use std::sync::Arc;

use bdk::bitcoin::secp256k1::PublicKey;
use futures::{StreamExt, TryFutureExt, TryStreamExt};
use futures::FutureExt;
// use libp2p::Multiaddr;
use log::{error, info};
use metrics::counter;
use tokio::runtime::Runtime;
use tokio::select;
use tokio::task::JoinHandle;
use tracing::debug;

use redgold_schema::{error_info, ErrorInfoContext, json_or, SafeOption};
use redgold_schema::errors::EnhanceErrorInfo;
use redgold_schema::structs::{ErrorInfo, NetworkEnvironment, NodeMetadata, PeerMetadata};

use crate::api::RgHttpClient;
use crate::core::internal_message::{PeerMessage, SendErrorInfo};
use crate::core::peer_rx_event_handler::rest_peer;
use crate::core::relay::Relay;
use crate::node_config::NodeConfig;
use crate::schema::json;
use crate::schema::structs::{Response, ResponseMetadata};
use crate::util;
// use crate::util::{to_libp2p_peer_id, to_libp2p_peer_id_ser};

use redgold_schema::EasyJson;
use redgold_schema::util::lang_util::SameResult;
use crate::util::logging::Loggable;

#[derive(Clone)]
pub struct PeerOutgoingEventHandler {
    // pub(crate) p2p_client: crate::api::p2p_io::rgnetwork::Client,
    relay: Relay,
    // rt: Arc<Runtime>
}

impl PeerOutgoingEventHandler {

    async fn send_peer_message(relay: Relay, message: PeerMessage) -> Result<(), ErrorInfo> {
        counter!("redgold.peer.send").increment(1);
        let ser_msgp = json_or(&message.request.clone());
        // tracing::info!("PeerOutgoingEventHandler send message {}", ser_msgp);
        if let Some(pk) = &message.public_key {
            let res = relay.ds.peer_store.query_public_key_metadata(&pk).await?;
            // TODO if metadata known, check if udp is required
            if let Some(nmd) = res {
                Self::send_message_rest(message.clone(), nmd, &relay).await?;
            } else {
                error!("Node metadata not found for peer public key to send message to {} contents: {}", pk.hex_or(), ser_msgp);
            }
        } else if let Some(nmd) = &message.node_metadata {
            debug!("PeerOutgoingEventHandler send message to node metadata {} with public key unregistered {}",
                nmd.json_or(),
                ser_msgp
            );
            Self::send_message_rest(message.clone(), nmd.clone(), &relay).await?;
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

    pub async fn send_message_rest(mut message: PeerMessage, nmd: NodeMetadata, relay: &Relay) -> Result<(), ErrorInfo> {
        counter!("redgold.peer.rest.send").increment(1);
        let result = match tokio::time::timeout(
            message.send_timeout.clone(), Self::send_message_rest_ret_err(&mut message, nmd, relay)
        ).await
            .error_info(
                format!("Timeout sending message to peer with duration {} secs",
                        message.send_timeout.as_secs())
            ) {
            Ok(r) => {r}
            Err(e) => {
                counter!("redgold.peer.rest.send.timeout").increment(1);
                Err(e)
            }
        };
        let r = result.map_err(|e| {
            counter!("redgold.peer.rest.send.error").increment(1);
            log::error!("Error sending message to peer: {}", json_or(&e));
            Response::from_error_info(e)
        }).combine();
        if let Some(response_channel) = &message.response {
            response_channel.send_err(r).add("Error sending message back on response channel").log_error().ok();
        }
        Ok(())
    }
    pub async fn send_message_rest_ret_err(message: &mut PeerMessage, nmd: NodeMetadata, relay: &Relay) -> Result<Response, ErrorInfo> {
        let port = nmd.port_or(relay.node_config.network) + 1;
        let res = rest_peer(
            relay, nmd.external_address()?.clone(), port as i64, &mut message.request
        ).await;
        res
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
    // async fn send_message_libp2p(&mut self, message: &PeerMessage, pd: &PeerMetadata) -> Result<Response, ErrorInfo> {
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
