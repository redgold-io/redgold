use crossbeam::atomic::AtomicCell;
use std::sync::Arc;

use crate::core::internal_message;
use crate::core::internal_message::Channel;
use crate::schema::structs::{
    Error, ErrorInfo, NodeState, PeerData, SubmitTransactionRequest, SubmitTransactionResponse,
};
use dashmap::DashMap;

use crate::core::internal_message::PeerMessage;
use crate::core::internal_message::RecvAsyncErrorInfo;
use crate::core::internal_message::TransactionMessage;
use crate::core::process_transaction::{RequestProcessor, UTXOContentionPool};
use crate::data::data_store::DataStore;
use crate::node_config::NodeConfig;
use crate::schema::structs::{Observation, ObservationMetadata};
use crate::schema::{ProtoHashable, SafeOption, WithMetadataHashable};
use crate::util;

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
pub struct Relay {
    pub node_config: NodeConfig,
    pub transaction: Channel<TransactionMessage>,
    pub observation: Channel<Observation>,
    pub observation_metadata: Channel<ObservationMetadata>,
    pub peer_message_tx: Channel<PeerMessage>,
    pub peer_message_rx: Channel<PeerMessage>,
    pub ds: DataStore,
    pub transaction_channels: Arc<DashMap<Vec<u8>, RequestProcessor>>,
    pub utxo_channels: Arc<DashMap<(Vec<u8>, i64), UTXOContentionPool>>,
    pub transaction_errors: Arc<DashMap<Vec<u8>, TransactionErrorCache>>,
    pub trust: Channel<TrustUpdate>,
    pub node_state: Arc<AtomicCell<NodeState>>,
}

/**
Deliberately unclone-able structure that tracks strict unshared dependencies which
are instantiated by the node
*/
pub struct StrictRelay {}
// Relay should really construct a bunch of non-clonable channels and return that data
// as the other 'half' here.
impl Relay {
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
            observation_metadata: internal_message::new_channel::<ObservationMetadata>(),
            peer_message_tx: internal_message::new_channel::<PeerMessage>(),
            peer_message_rx: internal_message::new_channel::<PeerMessage>(),
            ds,
            transaction_channels: Arc::new(DashMap::new()),
            utxo_channels: Arc::new(DashMap::new()),
            transaction_errors: Arc::new(Default::default()),
            trust: internal_message::new_channel::<TrustUpdate>(),
            node_state: Arc::new(AtomicCell::new(NodeState::Initializing)),
        }
    }
}

// https://doc.rust-lang.org/book/ch15-04-rc.html
