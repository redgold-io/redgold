use std::cmp::Ordering;
use std::collections::BinaryHeap;
use async_trait::async_trait;
use flume::{SendError, TrySendError};
use itertools::Itertools;
use redgold_schema::{error_info, error_message, RgResult, WithMetadataHashable};
use redgold_schema::seeds::get_seeds;
use redgold_schema::structs::Address;
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
struct MempoolEntry {
    transaction: TransactionMessage,
    fee_acceptable_address: Vec<Address>
}

#[async_trait]
impl IntervalFold for Mempool {
    async fn interval_fold(&mut self) -> RgResult<()> {
        let messages = self.relay.mempool.recv_while()?;
        let addrs = get_seeds().iter()
            .filter_map(|s| s.public_key.as_ref())
            .filter_map(|s| s.address().ok())
            .collect_vec();
        for message in messages {
            let entry = MempoolEntry {
                transaction: message,
                fee_acceptable_address: addrs.clone()
            };
            self.heap.push(entry);
        }
        loop {
            let option = self.heap.pop();
            if let Some(entry) = option {
                match self.relay.transaction_process.sender.try_send(entry.transaction.clone()) {
                    Ok(_) => {}
                    Err(e) => {
                        match e {
                            TrySendError::Full(_) => {
                                self.heap.push(entry);
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