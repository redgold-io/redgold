use std::collections::HashMap;
use crossbeam::atomic::AtomicCell;
use std::sync::Arc;
use std::time::Duration;

use crate::core::internal_message;
use crate::core::internal_message::Channel;
use crate::schema::structs::{
    Error, ErrorInfo, NodeState, PeerData, SubmitTransactionRequest, SubmitTransactionResponse,
};
use dashmap::DashMap;
use futures::future;
use futures::stream::FuturesUnordered;
use futures::task::SpawnExt;
use itertools::Itertools;
use log::info;
use tokio::runtime::Runtime;
use redgold_schema::{error_info, ErrorInfoContext, RgResult, structs};
use redgold_schema::errors::EnhanceErrorInfo;
use redgold_schema::structs::{AboutNodeRequest, FixedUtxoId, GossipTransactionRequest, Hash, MultipartySubscribeEvent, MultipartyThresholdRequest, MultipartyThresholdResponse, NodeMetadata, ObservationProof, Request, Response, Transaction};
use crate::core::discovery::DiscoveryMessage;

use crate::core::internal_message::PeerMessage;
use crate::core::internal_message::RecvAsyncErrorInfo;
use crate::core::internal_message::TransactionMessage;
use crate::core::process_transaction::{RequestProcessor, UTXOContentionPool};
use crate::data::data_store::DataStore;
use crate::node_config::NodeConfig;
use crate::schema::structs::{Observation, ObservationMetadata};
use crate::schema::{ProtoHashable, SafeOption, WithMetadataHashable};
use crate::util;
use crate::util::keys::ToPublicKey;

#[derive(Clone)]
pub struct TransactionErrorCache {
    pub process_time: u64,
    pub error: Error,
}

#[derive(Clone)]
pub struct TrustUpdate {
    pub update: PeerData,
    pub remove_peer: Option<Vec<u8>>,
}

#[derive(Clone)]
pub struct MultipartyRequestResponse {
    pub request: Option<MultipartyThresholdRequest>,
    pub response: Option<MultipartyThresholdResponse>,
    pub sender: Option<flume::Sender<MultipartyThresholdResponse>>,
    pub origin: Option<NodeMetadata>,
    pub internal_subscribe: Option<MultipartyRoomInternalSubscribe>
}

impl MultipartyRequestResponse {

    pub fn empty() -> Self {
        Self {
            request: None,
            response: None,
            sender: None,
            origin: None,
            internal_subscribe: None,
        }
    }
}

#[derive(Clone)]
pub struct MultipartyRoomInternalSubscribe {
    pub room_id: String,
    pub sender: flume::Sender<MultipartySubscribeEvent>
}

#[derive(Clone)]
pub struct ObservationMetadataInternalSigning {
    pub observation_metadata: ObservationMetadata,
    pub sender: flume::Sender<ObservationProof>
}

#[derive(Clone)]
pub struct Relay {
    /// Internal configuration
    pub node_config: NodeConfig,
    /// Incoming transactions
    pub transaction: Channel<TransactionMessage>,
    /// Externally received observations TODO: Merge this into transaction
    pub observation: Channel<Observation>,
    /// Threshold encryption multiparty signing flow
    pub multiparty: Channel<MultipartyRequestResponse>,
    /// Internal signing stream for handling some validated data that is to be observed and signed
    pub observation_metadata: Channel<ObservationMetadataInternalSigning>,
    /// Incoming interface for receiving messages from other peers
    pub peer_message_tx: Channel<PeerMessage>,
    /// Outgoing interface for sending messages to other peers
    pub peer_message_rx: Channel<PeerMessage>,
    /// Internal persistent data storage on disk, main access instance for everything persistence
    /// related
    pub ds: DataStore,
    /// All transactions currently in process, this needs to incorporate a priority mempool
    /// And be updated to deal with priority queue + persisted processing transactions
    pub transaction_channels: Arc<DashMap<Hash, RequestProcessor>>,
    /// TODO: This really needs to incorporate some kind of UTXO stream handler?
    pub utxo_channels: Arc<DashMap<FixedUtxoId, UTXOContentionPool>>,
    /// Some update associated with the trust model or change in rating label
    pub trust: Channel<TrustUpdate>,
    /// This isn't really used anywhere, but might be useful for keeping track of some kind of
    /// state information, most operations are actually designed to avoid any dependence on this
    /// but maybe useful for information purposes / observability
    pub node_state: Arc<AtomicCell<NodeState>>,
    /// Channel for outgoing messages over UDP, this streams to the UDP stream socket
    /// This should only be used internally by outgoing peer handler
    pub udp_outgoing_messages: Channel<PeerMessage>,
    /// Haha, discovery channel
    /// Used for immediate discovery messages for unknown or unrecognized message
    pub discovery: Channel<DiscoveryMessage>,
}

/**
Deliberately unclone-able structure that tracks strict unshared dependencies which
are instantiated by the node
*/

use crate::core::internal_message::SendErrorInfo;
use crate::core::peer_rx_event_handler::PeerRxEventHandler;

pub struct StrictRelay {}
// Relay should really construct a bunch of non-clonable channels and return that data
// as the other 'half' here.
impl Relay {

    pub async fn observe(&self, mut om: ObservationMetadata) -> Result<ObservationProof, ErrorInfo> {
        om.with_hash();
        let (sender, r) = flume::unbounded::<ObservationProof>();
        let omi = ObservationMetadataInternalSigning {
            observation_metadata: om,
            sender,
        };
        self.observation_metadata.sender.send_err(omi)?;
        let res = tokio::time::timeout(
            Duration::from_secs(self.node_config.observation_formation_millis.as_secs() + 10),
            r.recv_async_err()
        ).await.error_info("Timeout waiting for internal observation formation")??;
        Ok(res)
    }

    // TODO: add timeout
    pub async fn send_message_sync(&self, request: Request, node: structs::PublicKey, timeout: Option<Duration>) -> Result<Response, ErrorInfo> {
        let timeout = timeout.unwrap_or(Duration::from_secs(60));
        let (s, r) = flume::unbounded::<Response>();
        let mut pm = PeerMessage::from_pk(&request, &node.clone());
        pm.response = Some(s);
        self.peer_message_tx.sender.send_err(pm)?;
        let res = tokio::time::timeout(timeout, r.recv_async_err()).await
            .map_err(|e| error_info(e.to_string()))??;
        Ok(res)
    }

    pub async fn send_message_sync_static(relay: Relay, request: Request, node: structs::PublicKey, timeout: Option<Duration>) -> Result<Response, ErrorInfo> {
        let timeout = timeout.unwrap_or(Duration::from_secs(60));
        let (s, r) = flume::bounded::<Response>(1);
        let mut pm = PeerMessage::from_pk(&request, &node.clone());
        pm.response = Some(s);
        relay.peer_message_tx.sender.send_err(pm)?;
        let res = tokio::time::timeout(timeout, r.recv_async_err()).await
            .map_err(|e| error_info(e.to_string()))??;
        res.as_error_info()?;
        Ok(res)
    }

    pub async fn receive_message_sync(&self, request: Request, timeout: Option<Duration>) -> Result<Response, ErrorInfo> {
        // let key = request.verify_auth()?;
        let timeout = timeout.unwrap_or(Duration::from_secs(120));
        let (s, r) = flume::bounded::<Response>(1);
        let key = request.proof.clone().and_then(|p| p.public_key);
        let mut pm = PeerMessage::empty();
        pm.request = request;
        pm.response = Some(s);
        pm.public_key = key;
        self.peer_message_rx.sender.send_err(pm).add("receive_message_sync")?;
        let res = r.recv_async_err_timeout(timeout).await?;
        Ok(res)
    }

    pub async fn gossip(&self, tx: &Transaction) -> Result<(), ErrorInfo> {
        let all = self.ds.peer_store.select_gossip_peers(tx).await?;
        for p in all {
            let mut req = Request::default();
            let mut gtr = GossipTransactionRequest::default();
            gtr.transaction = Some(tx.clone());
            req.gossip_transaction_request = Some(gtr);
            self.send_message(req, p).await?;
        }
        Ok(())
    }

    pub async fn gossip_req(&self, req: &Request, hash: &Hash) -> Result<(), ErrorInfo> {
        let all = self.ds.peer_store.select_gossip_peers_hash(hash).await?;
        for p in all {
            self.send_message(req.clone(), p).await?;
        }
        Ok(())
    }

    // the new function everything should use
    pub async fn broadcast_async(
        &self,
        nodes: Vec<structs::PublicKey>,
        request: Request,
        timeout: Option<Duration>
    ) -> RgResult<Vec<RgResult<Response>>> {
        let mut results = vec![];
        for p in nodes {
            let mut req = request.clone();
            let res = self.send_message_async(req, p).await?;
            results.push(res);
        }
        let mut responses = vec![];
        for r in &results {
            let x = r
                .recv_async_err_timeout(timeout.unwrap_or(Duration::from_secs(20)));
            responses.push(x);
        }
        let res = futures::future::join_all(responses).await;
        Ok(res)
    }

    pub async fn broadcast(
        relay: Relay,
        nodes: Vec<structs::PublicKey>,
        request: Request,
        // runtime: Arc<Runtime>,
        timeout: Option<Duration>
        // TODO: remove the publickey here not necessary
    ) -> Vec<(structs::PublicKey, Result<Response, ErrorInfo>)> {
        let timeout = timeout.unwrap_or(Duration::from_secs(20));
        // let mut fu = FuturesUnordered::new();
        let mut fu = vec![];
        for (_,node) in nodes.iter().enumerate() {
            let relay2 = relay.clone();
            // let runtime2 = runtime.clone();
            let request2 = request.clone();
            let jh = async move {
                (
                node.clone(),
                {

                    tokio::spawn(
                        Relay::send_message_sync_static(relay2.clone(),
                                                        request2.clone(), node.clone(), Some(timeout))
                    ).await.error_info("join handle failure on broadcast").and_then(|e| e)
                }
            )};
            fu.push(jh);
        }

        future::join_all(fu).await
    }

    pub async fn send_message(&self, request: Request, node: structs::PublicKey) -> Result<(), ErrorInfo> {
        let pm = PeerMessage::from_pk(&request, &node);
        self.peer_message_tx.sender.send_err(pm)?;
        Ok(())
    }


    pub async fn send_message_async(&self, request: Request, node: structs::PublicKey) -> Result<flume::Receiver<Response>, ErrorInfo> {
        let (s, r) = flume::bounded(1);
        let mut pm = PeerMessage::from_pk(&request, &node);
        pm.response = Some(s);
        self.peer_message_tx.sender.send_err(pm)?;
        Ok(r)
    }

    pub async fn send_message_async_pm(&self, pm: PeerMessage) -> Result<flume::Receiver<Response>, ErrorInfo> {
        let (s, r) = flume::bounded(1);
        self.peer_message_tx.send(pm).await?;
        Ok(r)
    }


    pub async fn send_message_sync_pm(&self, mut pm: PeerMessage, timeout: Option<Duration>) -> RgResult<Response> {
        let (s, r) = flume::bounded(1);
        pm.response = Some(s);
        self.peer_message_tx.send(pm).await?;
        r.recv_async_err_timeout(timeout.unwrap_or(Duration::from_secs(20))).await
    }


    pub async fn submit_transaction_sync(
        &self,
        tx: &Transaction,
    ) -> Result<SubmitTransactionResponse, ErrorInfo> {
        self.submit_transaction(SubmitTransactionRequest{
            transaction: Some(tx.clone()),
            sync_query_response: true,
        }).await
    }

    pub async fn submit_transaction(
        &self,
        tx_req: SubmitTransactionRequest,
    ) -> Result<SubmitTransactionResponse, ErrorInfo> {
        let (s, r) = flume::bounded(1);
        let response_channel = if tx_req.sync_query_response {
            Some(s)
        } else {
            None
        };
        let tx = tx_req
            .transaction
            .safe_get_msg("Missing transaction field on submit request")?;
        tx.calculate_hash();
        // info!("Relay submitting transaction");
        self.transaction
            .send(TransactionMessage {
                transaction: tx.clone(),
                response_channel,
            })
            .await?;

        let mut response = SubmitTransactionResponse {
            transaction_hash: tx.clone().hash_or().into(),
            query_transaction_response: None,
            transaction: Some(tx.clone()),
        };
        if tx_req.sync_query_response {
            let response1 = r.recv_async_err().await?;
            response1.as_error_info()?;
            response = response1.submit_transaction_response.safe_get()?.clone();
            return Ok(response);
        }
        Ok(response)
    }

    pub async fn default() -> Self {
        Self::new(NodeConfig::default_debug()).await
    }
    pub async fn new(node_config: NodeConfig) -> Self {
        // Inter thread processes
        let ds = DataStore::from_config(&node_config.clone()).await;
        Self {
            node_config,
            transaction: internal_message::new_channel::<TransactionMessage>(),
            observation: internal_message::new_channel::<Observation>(),
            multiparty: internal_message::new_channel::<MultipartyRequestResponse>(),
            observation_metadata: internal_message::new_channel::<ObservationMetadataInternalSigning>(),
            peer_message_tx: internal_message::new_channel::<PeerMessage>(),
            peer_message_rx: internal_message::new_channel::<PeerMessage>(),
            ds,
            transaction_channels: Arc::new(DashMap::new()),
            utxo_channels: Arc::new(DashMap::new()),
            trust: internal_message::new_channel::<TrustUpdate>(),
            node_state: Arc::new(AtomicCell::new(NodeState::Initializing)),
            udp_outgoing_messages: internal_message::new_channel::<PeerMessage>(),
            discovery: internal_message::new_bounded_channel(100),
        }
    }
}

// https://doc.rust-lang.org/book/ch15-04-rc.html
