use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use std::sync::Arc;

use bdk::bitcoin::secp256k1::PublicKey;
use futures::FutureExt;
use futures::{StreamExt, TryFutureExt, TryStreamExt};
use metrics::counter;
use redgold_common::flume_send_help::SendErrorInfo;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::{DynamicNodeMetadata, ErrorInfo, NetworkEnvironment, NodeMetadata, PeerMetadata, TransportBackend};
use redgold_schema::message::Request;
use redgold_schema::{error_info, message, structs, ErrorInfoContext, RgResult, SafeOption};
use tokio::runtime::Runtime;
use tokio::select;
use tokio::task::JoinHandle;
use tracing::debug;
// use libp2p::Multiaddr;
use tracing::{error, info};

use redgold_common::client::http::RgHttpClient;
use crate::core::internal_message::PeerMessage;
use crate::core::relay::Relay;
use crate::schema::structs::{ ResponseMetadata};
use crate::schema::message::{Response};
use crate::util;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::conf::port_offsets::PortOffsetHelpers;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::helpers::easy_json::json;
// use crate::util::{to_libp2p_peer_id, to_libp2p_peer_id_ser};

use redgold_schema::helpers::easy_json::json_or;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::util::lang_util::{SameResult, WithMaxLengthString};

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
                if !Self::maybe_udp_message(&relay, &message, pk, &nmd).await? {
                    Self::send_message_rest(message.clone(), nmd, &relay).await?;
                }
            } else {
                // error!("Node metadata not found for peer public key to send message to {} contents: {}", pk.hex(), ser_msgp);
            }
        } else if let Some(nmd) = &message.node_metadata {
            // debug!("PeerOutgoingEventHandler send message to node metadata {} with public key unregistered {}",
            //     nmd.json_or(),
            //     ser_msgp
            // );
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

    async fn maybe_udp_message(relay: &Relay, message: &PeerMessage, pk: &structs::PublicKey, nmd: &NodeMetadata) -> RgResult<bool> {
        let mut message = message.clone();
        if let Some(ti) = nmd.transport_info.as_ref() {
            let nat_restricted = ti.nat_restricted.unwrap_or(false);
            if message.requested_transport == Some(TransportBackend::Udp) || nat_restricted {
                let dynamic_udp_port = relay.peer_info.read_dynamic(pk).and_then(|d| d.udp_port);
                if nat_restricted && dynamic_udp_port.is_none() {
                    return "Nat restricted and no dynamic UDP port available".to_error()
                }
                let offset = dynamic_udp_port.unwrap_or(nmd.port_or(relay.node_config.network.clone()) as i64);
                let udp_port = offset.udp_port();
                if let Some(ip) = ti.external_ipv4.clone() {
                    let ip = Ipv4Addr::from_str(&*ip).error_info("ip parse")?;
                    let socket_addr = SocketAddr::new(IpAddr::V4(ip), udp_port);
                    message.socket_addr = Some(socket_addr);
                    relay.udp_outgoing_messages.send(message).await?
                }
            }
        }
        Ok(false)
    }

    async fn run(&mut self) -> Result<(), ErrorInfo> {

        use futures::StreamExt;
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
            message.send_timeout.clone(), Self::send_message_rest_ret_err(&mut message, nmd.clone(), relay)
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

        let added_details = result.map_err(|e| {
            counter!("redgold.peer.rest.send.error").increment(1);
            let mut e2 = e.clone();
            e2.with_detail("node_metadata", nmd.json_or());
            e2.with_detail("message", message.request.json_or().with_max_length(1000));
            log::error!("Error sending message to peer: {}", e2.json_or());
            e2
        });

        if let Some(pk) = nmd.public_key.as_ref() {
            if let Err(e) = &added_details {
                relay.mark_peer_send_failure(pk, e).await?;
                // relay.discover_peer(&nmd).await.log_error().ok();
            } else {
                relay.mark_peer_send_success(pk).await?;
            }
        }

        let r = added_details.map_err(|e| {
            Response::from_error_info(e)
        }).combine();
        if let Some(response_channel) = &message.response {
            response_channel.send_rg_err(r).add("Error sending message back on response channel").log_error().ok();
        }
        Ok(())
    }

    pub async fn send_message_rest_ret_err(message: &mut PeerMessage, nmd: NodeMetadata, relay: &Relay) -> Result<message::Response, ErrorInfo> {
        let port = nmd.port_or(relay.node_config.network) + 1;
        let request = message.request.clone();
        let pk = nmd.public_key.safe_get_msg("Missing public key on node metadata in outgoing request")?;
        let res = rest_peer(
            relay, nmd.external_address()?.clone(), port as i64, request, pk
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

pub async fn rest_peer(relay: &Relay, ip: String, port: i64, request: Request, intended_pk: &structs::PublicKey) -> Result<Response, ErrorInfo> {
    let client = redgold_common::client::http::RgHttpClient::new(ip, port as u16, Some(Box::new(relay.clone())));
    client.proto_post_request(request, Some(relay.node_metadata().await?), Some(intended_pk)).await
}
