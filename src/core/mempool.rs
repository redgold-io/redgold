use std::cmp::Ordering;
use std::collections::BinaryHeap;
use async_trait::async_trait;
use flume::{SendError, TrySendError};
use itertools::Itertools;
use redgold_schema::{error_info, error_message, RgResult, WithMetadataHashable};
use redgold_schema::structs::{Address, QueryTransactionResponse, Response, SubmitTransactionResponse, Transaction};
use crate::core::internal_message::{SendErrorInfo, TransactionMessage};
use crate::core::relay::Relay;
use crate::core::stream_handlers::IntervalFold;

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
    pub transaction: TransactionMessage,
    pub fee_acceptable_address: Vec<Address>
}

#[async_trait]
impl IntervalFold for Mempool {
    async fn interval_fold(&mut self) -> RgResult<()> {
        let messages = self.relay.mempool.recv_while()?;
        let addrs = self.relay.node_config.seeds.iter()
            .filter_map(|s| s.public_key.as_ref())
            .filter_map(|s| s.address().ok())
            .collect_vec();
        for message in messages {
            let h = message.transaction.hash_or();
            let is_known = self.relay.transaction_known(&h).await?;
            if is_known {
                if let Some(r) = message.response_channel {
                    r.send_rg_err(Response::from_error_info(error_info("Transaction already in process or known")))?;
                }
                // TODO: Add a subscriber to relay and at end of transaction process notify all subscribers
                // Notify subscribers for transaction channel rather than just dropping and returning error
                continue
            }

            let entry = MempoolEntry {
                transaction: message,
                fee_acceptable_address: addrs.clone()
            };
            self.push(entry);
        }
        loop {
            let option = self.pop();
            if let Some(entry) = option {
                match self.relay.transaction_process.sender.try_send(entry.transaction.clone()) {
                    Ok(_) => {}
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