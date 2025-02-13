use crate::core::internal_message::TransactionMessage;
use crate::core::relay::Relay;
use crate::core::transact::tx_validate::TransactionValidator;
use crate::util;
use async_trait::async_trait;
use flume::{SendError, TrySendError};
use futures::FutureExt;
use itertools::Itertools;
use metrics::{counter, gauge};
use redgold_common::flume_send_help::SendErrorInfo;
use redgold_common_no_wasm::stream_handlers::IntervalFold;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::tx_proof_validate::TransactionProofValidator;
use redgold_schema::fee_validator::TransactionFeeValidator;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::pow::TransactionPowValidate;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{Address, QueryTransactionResponse, SubmitTransactionResponse, Transaction};
use redgold_schema::message::Response;
use redgold_schema::{error_info, error_message, RgResult};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use tracing::Level;

pub struct Mempool {
    relay: Relay,
    heap: BinaryHeap<MempoolEntry>
}


impl Mempool {
    pub fn new(relay: &Relay) -> Self {
        Self {
            relay: relay.clone(),
            heap: BinaryHeap::new()
        }
    }
    pub fn push(&mut self, mempool_entry: MempoolEntry) {
        let transaction = mempool_entry.transaction.transaction.clone();
        self.heap.push(mempool_entry);
        self.relay.mempool_entries.insert(transaction.hash_or(), transaction);
    }

    pub fn pop(&mut self) -> Option<MempoolEntry> {
        let o = self.heap.pop();
        if let Some(me) = &o {
            self.relay.mempool_entries.remove(&me.transaction.transaction.hash_or());
        }
        o
    }

    async fn verify_and_form_entry(&mut self, addrs: &Vec<Address>, message: &TransactionMessage) -> RgResult<MempoolEntry> {
        let h = message.transaction.hash_or();
        let is_known = self.relay.transaction_known(&h).await.mark_abort()?;
        if is_known {
            Err(error_info("Transaction already in process or known"))
                .mark_skip_log()?
            // TODO: Add a subscriber to relay and at end of transaction process notify all subscribers
            // Notify subscribers for transaction channel rather than just dropping and returning error
        }
        message.transaction.validate(Some(addrs), Some(&self.relay.node_config.network))?;

        let entry = MempoolEntry {
            transaction: message.clone()
        };
        Ok(entry)
    }
}

impl Eq for MempoolEntry {}

impl PartialEq<Self> for MempoolEntry {
    fn eq(&self, other: &Self) -> bool {
        other.transaction.transaction.hash_or().eq(&self.transaction.transaction.hash_or())
    }
}

impl PartialOrd<Self> for MempoolEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // TODO: Compare by seed fee acceptable ordering
        other.transaction.transaction.total_output_amount().partial_cmp(
            &self.transaction.transaction.total_output_amount())
    }
}

impl Ord for MempoolEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cmp(other)
    }
}

#[derive(Debug, Clone)]
pub struct MempoolEntry {
    pub transaction: TransactionMessage
}


#[async_trait]
impl IntervalFold for Mempool {
    async fn interval_fold(&mut self) -> RgResult<()> {

        // TODO: Construct mempool instance with this information already known
        // dynamic query here is unnecessary overhead.
        let genesis_hash = self.relay.ds.config_store.get_genesis().await?.map(|g| g.hash_or());

        let messages = self.relay.mempool.recv_while()?;
        gauge!("redgold_mempool_messages_recv").set(self.heap.len() as f64);

        let addrs = self.relay.node_config.seed_peer_addresses();
        for message in messages {
            let m_hash = message.transaction.hash_or();
            if let Some(h) = genesis_hash.as_ref() {
                if h.eq(&m_hash) {
                    continue;
                }
            }
            match self.verify_and_form_entry(&addrs, &message)
                .await
                .with_detail("transaction", message.transaction.json_or())
                .with_detail("public_key_origin", message.origin.json_or())
                .with_detail("origin_ip", message.origin_ip.json_or())
                .with_detail("transaction_hash", message.transaction.hash_hex())
                .with_detail("genesis?", genesis_hash.json_or())
                .log_error()
                .bubble_abort()? {
                Err(e) => {
                    counter!("redgold_mempool_rejected").increment(1);
                    // TODO: Why does this break the E2E test?
                    // self.relay.ds.accept_transaction(
                    //     &message.transaction, util::current_time_millis_i64(), Some(e.clone()), false
                    // ).await?;
                    if let Some(r) = message.response_channel {
                        r.send_rg_err(Response::from_error_info(e))?;
                    }
                }
                Ok(entry) => {
                    counter!("redgold_mempool_added").increment(1);
                    self.push(entry);
                }
            }
        }
        gauge!("redgold_mempool_size").set(self.heap.len() as f64);

        loop {
            let option = self.pop();
            if let Some(entry) = option {
                counter!("redgold_mempool_pop").increment(1);
                match self.relay.transaction_process.sender.try_send(entry.transaction.clone()) {
                    Ok(_) => {
                        counter!("redgold_mempool_sent").increment(1);
                    }
                    Err(e) => {
                        match e {
                            TrySendError::Full(_) => {
                                self.push(entry);
                                break;
                            }
                            TrySendError::Disconnected(_) => {
                                return Err(error_info("transaction process channel closed unexpectedly"));
                            }
                        }
                    }
                }
            } else {
                break;
            }
        }

        Ok(())
    }
}