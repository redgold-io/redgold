use std::collections::HashSet;
use std::time::Duration;
use async_trait::async_trait;
use futures::TryFutureExt;
// use libp2p::request_response::RequestResponseMessage::Request;
use log::info;
use metrics::increment_counter;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::IntervalStream;
use tracing::{debug, error};
use redgold_schema::{RgResult, SafeOption, structs};
use redgold_schema::errors::EnhanceErrorInfo;
use redgold_schema::structs::{DynamicNodeMetadata, ErrorInfo, GetPeersInfoRequest, NodeMetadata, Response};
use crate::core::relay::Relay;
use crate::core::stream_handlers::{IntervalFold, RecvForEachConcurrent};
use crate::e2e::run;
use crate::util::logging::Loggable;
use redgold_schema::EasyJson;
use crate::core::internal_message::{PeerMessage, RecvAsyncErrorInfo};

/**
Big question here is should discovery happen as eager push on Observation buffer
or both?

Probably both.
*/


#[async_trait]
impl IntervalFold for Discovery {
    async fn interval_fold(&mut self) -> RgResult<()> {
        let mut peers = self.relay.ds.peer_store
            .active_nodes(None).await
            .add("Active nodes query in discovery failure")?;

        // debug!("Running discovery for {} stored peers", peers.len());
        let mut results = HashSet::new();

        let mut req = structs::Request::default();
        req.get_peers_info_request = Some(GetPeersInfoRequest::default());
        for (r, pk) in self.relay.broadcast_async(
            peers.clone(), req, None).await?.iter().zip(peers.clone()) {
            match r {
                Ok(o) => {
                    if let Some(o) = &o.get_peers_info_response {
                        // TODO: There's probably a better way to merge peer info here.
                        // Problem here is we might have slightly different but almost same based
                        // on observation ordinal
                        // o.peer_info
                        results.extend(o.peer_info.clone())
                    }
                    let current_key = o.node_metadata.as_ref().and_then(|n| n.public_key.as_ref());
                    if let Some(ck) = current_key {
                        if ck != &pk {
                            error!("Discovery response public key does not match request public key");
                            self.relay.ds.peer_store.remove_node(&pk).await?;
                        }
                    }
                }
                Err(e) => {
                    error!("Error in discovery: {}", e.json_or());
                    self.relay.ds.peer_store.remove_node(&pk).await?;
                }
            }
        }

        // debug!("Discovery found {} total peers", results.len());
        let mut new_peers_count = 0;
        for r in &results {
            if let Some(pk) = r.latest_node_transaction.clone()
                .and_then(|t| t.node_metadata().ok())
                .and_then(|t| t.public_key){
                if pk != self.relay.node_config.public_key() {

                    let known = self.relay.ds.peer_store.query_public_key_node(&pk).await?.is_some();
                    if !known {
                        debug!("Discovery invoking database add for new peer {}", pk.hex().expect("hex"));
                        // TODO: we need to validate this peerNodeInfo first BEFORE adding it to peer store
                        // For now just dropping errors to log
                        // TODO: Query trust for this peerId first, before updating trust score.
                        // Security thing here needs to be fixed later.
                        self.relay.ds.peer_store.add_peer_new(r, 1f64, &self.relay.node_config.public_key()).await.log_error().ok();
                    }
                }
            } else {
                error!("Discovery found peer with no public key: {}", r.json_or())
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct DiscoveryMessage {
    pub node_metadata: NodeMetadata,
    pub dynamic_node_metadata: Option<DynamicNodeMetadata>
}

impl DiscoveryMessage {
    pub fn new(node_metadata: NodeMetadata, dynamic_node_metadata: Option<DynamicNodeMetadata>) -> Self {
        Self {
            node_metadata,
            dynamic_node_metadata
        }
    }
}

#[async_trait]
impl RecvForEachConcurrent<DiscoveryMessage> for Discovery {

    // TODO: Ensure discovery message is not for self
    async fn recv_for_each(&mut self, message: DiscoveryMessage) -> RgResult<()> {
        increment_counter!("redgold.peer.discovery.recv_for_each");
        let mut request = structs::Request::default();
        request.about_node_request = Some(structs::AboutNodeRequest::default());
        // message.dynamic_node_metadata
        let nmd = message.node_metadata.clone();
        let msg = PeerMessage::from_metadata(request, nmd);
        // Should we add metrics here on timeouts or some other way to handle repeatedly
        // making requests to a dead peer?
        // Maybe that should only really happen on the background process where we can track that internally in mem
        tracing::debug!("Sending discovery message to peer: {}", message.node_metadata.long_identifier());
        let result = self.relay.send_message_sync_pm(msg, None).await;
        let done = match result {
            Ok(r) => {
                let res = self.process(message.clone(), r).await;
                tracing::debug!("Got discovery response for peer: {}", message.node_metadata.long_identifier());
                res
            }
            Err(e) => {Err(e)}
        };
        done.log_error().ok();
        if done.is_ok() {
            tracing::debug!("Discovery success for peer: {}", message.node_metadata.long_identifier());
        }
        Ok(())
    }
}


#[derive(Clone)]
pub struct Discovery {
    relay: Relay
}

impl Discovery {
    pub async fn new(relay: Relay) -> Self {
        Self {
            relay
        }
    }

    async fn process(&mut self, message: DiscoveryMessage, result: Response) -> RgResult<()> {

        let result = result.about_node_response.safe_get_msg(
            "Missing about node response during peer discovery"
        );
        let res = result?.peer_node_info.safe_get_msg(
            "Missing about node response peer_node_info during peer discovery"
        )?;
        let nmd = res.latest_node_transaction.safe_get_msg(
            "Missing about node response latest_node_transaction during peer discovery"
        )?.node_metadata()?;
        let pk = nmd.public_key.safe_get_msg(
            "Missing about node response public_key during peer discovery"
        )?;
        let short_peer_id = pk.short_id();

        // TODO: Validate message and so on here.
        // Are we verifying auth on the response somewhere else?
        self.relay.ds.peer_store.add_peer_new(res, 1f64, &self.relay.node_config.public_key()).await?;
        tracing::debug!("Added new peer from immediate discovery: {}", short_peer_id);

        Ok(())
    }

}