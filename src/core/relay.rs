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
use itertools::Itertools;
use redgold_schema::{error_info, structs};
use redgold_schema::structs::{MultipartySubscribeEvent, MultipartyThresholdRequest, MultipartyThresholdResponse, NodeMetadata, Request, Response};

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
pub struct Relay {
    pub node_config: NodeConfig,
    pub transaction: Channel<TransactionMessage>,
    pub observation: Channel<Observation>,
    pub multiparty: Channel<MultipartyRequestResponse>,
    pub observation_metadata: Channel<ObservationMetadata>,
    pub peer_message_tx: Channel<PeerMessage>,
    pub peer_message_rx: Channel<PeerMessage>,
    pub ds: DataStore,
    pub transaction_channels: Arc<DashMap<Vec<u8>, RequestProcessor>>,
    pub utxo_channels: Arc<DashMap<(Vec<u8>, i64), UTXOContentionPool>>,
    pub transaction_errors: Arc<DashMap<Vec<u8>, TransactionErrorCache>>,
    pub trust: Channel<TrustUpdate>,
    pub node_state: Arc<AtomicCell<NodeState>>,
    pub udp_outgoing_messages: Channel<PeerMessage>
}

/**
Deliberately unclone-able structure that tracks strict unshared dependencies which
are instantiated by the node
*/

use crate::core::internal_message::SendErrorInfo;

pub struct StrictRelay {}
// Relay should really construct a bunch of non-clonable channels and return that data
// as the other 'half' here.
impl Relay {

    // TODO: add timeout
    pub async fn send_message_sync(&self, request: Request, node: structs::PublicKey, timeout: Option<Duration>) -> Result<Response, ErrorInfo> {
        let timeout = timeout.unwrap_or(Duration::from_secs(60));
        let (s, r) = flume::unbounded::<Response>();
        let key = node.to_public_key()?;
        let pm = PeerMessage{
            request,
            response: Some(s),
            public_key: Some(key),
            socket_addr: None,
        };
        self.peer_message_tx.sender.send_err(pm)?;
        let res = tokio::time::timeout(timeout, r.recv_async_err()).await
            .map_err(|e| error_info(e.to_string()))??;
        Ok(res)
    }

    pub async fn receive_message_sync(&self, request: Request, timeout: Option<Duration>) -> Result<Response, ErrorInfo> {
        let key = request.verify_auth()?;
        let timeout = timeout.unwrap_or(Duration::from_secs(60));
        let (s, r) = flume::unbounded::<Response>();
        let pm = PeerMessage{
            request,
            response: Some(s),
            public_key: Some(key.to_public_key()?),
            socket_addr: None,
        };
        self.peer_message_rx.sender.send_err(pm)?;
        let res = tokio::time::timeout(timeout, r.recv_async_err()).await
            .map_err(|e| error_info(e.to_string()))??;
        Ok(res)
    }

    pub async fn broadcast(&self, nodes: Vec<structs::PublicKey>, request: Request, timeout: Option<Duration>) -> Vec<(structs::PublicKey, Result<Response, ErrorInfo>)> {
        let timeout = timeout.unwrap_or(Duration::from_secs(20));
        // let mut fu = FuturesUnordered::new();
        let mut fu = vec![];
        for node in nodes {
            fu.push(async {
                (node.clone(), self.send_message_sync(request.clone(), node, Some(timeout)).await)
            });
        }
        future::join_all(fu).await
        // fu.collect_vec::<>().await
    }

    pub async fn send_message(&self, request: Request, node: structs::PublicKey) -> Result<(), ErrorInfo> {
        let key = node.to_public_key()?;
        let pm = PeerMessage{
            request,
            response: None,
            public_key: Some(key),
            socket_addr: None,
        };
        self.peer_message_tx.sender.send_err(pm)?;
        Ok(())
    }

    pub async fn submit_transaction(
        &self,
        tx_req: SubmitTransactionRequest,
    ) -> Result<SubmitTransactionResponse, ErrorInfo> {
        let (s, r) = flume::unbounded();
        let response_channel = if tx_req.sync_query_response {
            Some(s)
        } else {
            None
        };
        let tx = tx_req
            .transaction
            .safe_get_msg("Missing transaction field on submit request")?;
        self.transaction
            .send(TransactionMessage {
                transaction: tx.clone(),
                response_channel,
            })
            .await?;
        let mut response = SubmitTransactionResponse {
            transaction_hash: tx.clone().hash().into(),
            query_transaction_response: None,
        };
        if tx_req.sync_query_response {
            let res = r.recv_async_err().await?;
            response.query_transaction_response = res.query_transaction_response;
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
            observation_metadata: internal_message::new_channel::<ObservationMetadata>(),
            peer_message_tx: internal_message::new_channel::<PeerMessage>(),
            peer_message_rx: internal_message::new_channel::<PeerMessage>(),
            ds,
            transaction_channels: Arc::new(DashMap::new()),
            utxo_channels: Arc::new(DashMap::new()),
            transaction_errors: Arc::new(Default::default()),
            trust: internal_message::new_channel::<TrustUpdate>(),
            node_state: Arc::new(AtomicCell::new(NodeState::Initializing)),
            udp_outgoing_messages: internal_message::new_channel::<PeerMessage>()
        }
    }
}

// https://doc.rust-lang.org/book/ch15-04-rc.html
