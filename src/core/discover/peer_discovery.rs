use crate::core::internal_message::PeerMessage;
use crate::core::relay::{Relay, SafeLock};
use crate::util;
use async_trait::async_trait;
use futures::TryFutureExt;
use itertools::Itertools;
use metrics::counter;
use redgold_common::flume_send_help::RecvAsyncErrorInfo;
use redgold_common_no_wasm::stream_handlers::{IntervalFold, TryRecvForEach};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{DynamicNodeMetadata, ErrorInfo, GetPeersInfoRequest, NodeMetadata, PeerNodeInfo};
use redgold_schema::message::Response;
use redgold_schema::util::lang_util::WithMaxLengthString;
use redgold_schema::{message, structs, RgResult, SafeOption};
use std::collections::HashSet;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::IntervalStream;
use tracing::{debug, error};
// use libp2p::request_response::RequestResponseMessage::Request;
use tracing::{info, trace};
use redgold_schema::message::Request;

/**
Big question here is should discovery happen as eager push on Observation buffer
or both?

Probably both.
 */
#[async_trait]
impl IntervalFold for Discovery {
    async fn interval_fold(&mut self) -> RgResult<()> {

        self.clear_dead_peers().await?;

        // What happens if the peer is non-responsive?
        let node_tx_all = self.relay.ds.peer_store.active_node_info(None)
            .await
            .add("Active nodes query in discovery failure")?;

        let peers = node_tx_all.iter()
            .filter_map(|x| x.node_metadata().ok())
            .filter_map(|n| n.public_key)
            .collect_vec();

        assert_eq!(node_tx_all.len(), peers.len());
        // debug!("Running discovery for {} stored peers", peers.len());
        let mut results = HashSet::new();

        // Should we first query to make sure this node is still valid?
        // We need to make sure this hostname is unique, i.e. the stored peer we know about
        // Compare the data store against the actual node.
        let mut req = Request::default();
        req.get_peers_info_request = Some(GetPeersInfoRequest::default());
        for (r, node_tx_original) in self.relay.broadcast_async(
            peers.clone(), req, None).await?.iter().zip(node_tx_all.clone()) {
            match r {
                Ok(o) => {
                    if let Some(o) = &o.get_peers_info_response {
                        // TODO: There's probably a better way to merge peer info here.
                        // Problem here is we might have slightly different but almost same based
                        // on observation ordinal
                        // o.peer_info
                        results.extend(o.peer_info.clone());
                        let info: Option<&PeerNodeInfo> = o.self_info.as_ref();
                        if let Some(info) = info {
                            if let Some(latest_node_tx) = info.latest_node_transaction.as_ref() {
                                if latest_node_tx != &node_tx_original {
                                    error!("Discovery response node transaction does not match original, removing latest_node_tx {} node_tx_original {}", latest_node_tx.json_or(), node_tx_original.json_or());
                                    let pk_o = node_tx_original.node_metadata().expect("nmd").public_key.expect("pk");
                                    self.relay.ds.peer_store.remove_node(&pk_o).await?;
                                }
                            }
                            self.relay.ds.peer_store.add_peer_new(info,
                                                                  &self.relay.node_config.public_key()
                            ).await?;
                        }
                    }
                }
                Err(e) => {
                    error!("Error in discovery query: {} removing node", e.json_or());
                    let nmd = node_tx_original.node_metadata();
                    if let Ok(nmd) = nmd {
                        if let Some(key) = nmd.public_key.as_ref() {
                            info!("Removing node in discovery query with error: {} {}", key.hex(), nmd.json_or());
                            self.relay.ds.peer_store.remove_node(
                                &key
                            ).await?;
                        }
                    }

                }
            }
        }

        // debug!("Discovery found {} total peers", results.len());
        let _new_peers_count = 0;

        let potential_new_peers =
        for r in &results {
            if let Some((pk, nmd)) = r.latest_node_transaction.clone()
                .and_then(|t| t.node_metadata().ok())
                .and_then(|t| t.public_key.clone().map(|x| (x, t.clone()))) {
                if pk != self.relay.node_config.public_key() {
                    let known = self.relay.ds.peer_store.query_public_key_node(&pk).await?.is_some();
                    if !known {
                        // debug!("Batch Discovery sending discovery query to new peer {}", pk.hex());
                        // TODO: we need to validate this peerNodeInfo first BEFORE adding it to peer store
                        // For now just dropping errors to log
                        // TODO: Query trust for this peerId first, before updating trust score.
                        // Security thing here needs to be fixed later.
                        // self.relay.ds.peer_store.add_peer_new(r, &self.relay.node_config.public_key()).await.log_error().ok();
                        self.relay.discovery.send(DiscoveryMessage::new(nmd, None)).await?;
                    }
                }
            } else {
                error!("Discovery found peer with no public key: {}", r.json_or());
            }
        };
        Ok(())
    }
}

#[derive(Clone)]
pub struct DiscoveryMessage {
    pub node_metadata: NodeMetadata,
    pub dynamic_node_metadata: Option<DynamicNodeMetadata>,
}

impl DiscoveryMessage {
    pub fn new(node_metadata: NodeMetadata, dynamic_node_metadata: Option<DynamicNodeMetadata>) -> Self {
        Self {
            node_metadata,
            dynamic_node_metadata,
        }
    }
}

#[async_trait]
impl TryRecvForEach<DiscoveryMessage> for Discovery {
    // TODO: Ensure discovery message is not for self
    async fn try_recv_for_each(&mut self, message: DiscoveryMessage) -> RgResult<()> {
        counter!("redgold.peer.discovery.recv_for_each").increment(1);
        let mut request = message::Request::default();
        request.about_node_request = Some(structs::AboutNodeRequest::default());
        // message.dynamic_node_metadata
        let nmd = message.node_metadata.clone();
        let msg = PeerMessage::from_metadata(request, nmd);
        // Should we add metrics here on timeouts or some other way to handle repeatedly
        // making requests to a dead peer?
        // Maybe that should only really happen on the background process where we can track that internally in mem
        // tracing::debug!("Sending discovery message to peer: {}", message.node_metadata.long_identifier());
        let result = self.relay.send_message_sync_pm(msg, None).await;
        let done = match result {
            Ok(r) => {
                let res = self.process(message.clone(), r.clone()).await
                    .with_detail("long_identifier", message.node_metadata.long_identifier())
                    .with_detail("response", r.json_or().with_max_length(3000))
                    .with_detail("node_metadata", message.node_metadata.json_or());
                trace!("Got discovery response for peer: {} {}", message.node_metadata.long_identifier(), r.json_or().with_max_length(3000));
                res
            }
            Err(e) => {
                // Check to remove this peer as dead if it already existed before?
                info!("Error in discovery try_recv_for_each query: {} removing node {}", message.node_metadata.long_identifier(), e.json_or());
                if let Some(pk) = message.node_metadata.public_key.as_ref() {
                    self.relay.ds.peer_store.remove_node(&pk).await?;
                }
                // TODO: Tasklocal error handling
                Err(e).with_detail("long_identifier", message.node_metadata.long_identifier())
                    .with_detail("node_metadata", message.node_metadata.json_or())
            }
        };
        done.log_error().ok();
        if done.is_ok() {
            debug!("Discovery success for peer: {}", message.node_metadata.long_identifier());
        }
        Ok(())
    }
}


#[derive(Clone)]
pub struct Discovery {
    relay: Relay,
}

impl Discovery {
    pub async fn clear_dead_peers(&self) -> RgResult<()> {
        let ct = util::current_time_millis_i64();
        let failures = self.relay.peer_send_failures.safe_lock().await?;
        for (pk, fails) in failures.iter() {
            let delta = (ct - fails.1) / 1000;
            if delta > 60 * 5 { // 5 mins, 2 e2e intervals
                self.relay.ds.peer_store.remove_node(pk).await?;
                info!("Removed dead peer with delta seconds {}: {} last_err {}", delta, pk.hex(), fails.0.json_or());
                counter!("redgold.peer.discovery.clear_dead_peers").increment(1);
            }
        }
        Ok(())
    }
}

impl Discovery {
    pub async fn new(relay: Relay) -> Self {
        Self {
            relay
        }
    }

    async fn process(&mut self, _message: DiscoveryMessage, result: Response) -> RgResult<()> {
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
        self.relay.ds.peer_store.add_peer_new(res, &self.relay.node_config.public_key()).await?;
        // tracing::debug!("Added new peer from immediate discovery: {}", short_peer_id);

        Ok(())
    }
}