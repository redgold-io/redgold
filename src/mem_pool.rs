use dashmap::DashMap;

use crate::data::data_store::DataStore;
use crate::schema::structs::{Observation, Transaction};

#[allow(dead_code)]
pub struct ProvenObservation {
    pub peer_id: [u8; 32], // Merkle Proof of a peer's rotation keys.
    pub key: [u8; 32],
    pub confirmations: Vec<[u8; 32]>,
}
#[allow(dead_code)]
pub struct TransactionInfo {
    transaction: Transaction,
    receipt_time: u64,
    pub observations: Vec<ProvenObservation>, //processing_channel: Channel<TransactionProcessMessage>
}
#[allow(dead_code)]
pub struct MemPool {
    pub transactions: DashMap<[u8; 32], TransactionInfo>,
    observations: DashMap<[u8; 32], Observation>,
}

impl MemPool {
    #[allow(dead_code)]
    pub fn default() -> Self {
        Self {
            transactions: DashMap::new(),
            observations: DashMap::new(),
        }
    }
    #[allow(dead_code)]
    pub fn total_trust(&self, hash: &[u8; 32], data_store: &DataStore) -> Option<f64> {
        match self.transactions.get(&hash.clone()) {
            None => {
                return None;
                // log some kind of error here
            }
            Some(tx_info) => {
                let observations = &tx_info.value().clone().observations;
                let ids = observations
                    .iter()
                    .map(|o| o.peer_id.to_vec())
                    .collect::<Vec<Vec<u8>>>();
                // for obs in observations {
                //     //self.ds;
                //     obs.peer_id;
                //     // TODO: use confirmations as well
                //     // obs.key
                // }
                let map = data_store.select_peer_trust(&ids).unwrap();
                let trust = map.values().clone();
                let sum = trust.sum::<f64>();
                let avg: f64 = sum / (map.len() as f64);
                return Some(avg);
            }
        }
    }
    //
    // pub fn check_transaction_known2(
    //     &self,
    //     transaction_hash: &[u8; 32],
    //     transaction: &Transaction,
    // ) -> Result<(), Error> {
    //     //https://github.com/xacrimon/dashmap/issues/78
    //
    //     // actually shit the flow should be
    //     // is transaction in memPool?
    //     // if so ignore
    //     // is transaction referencing valid utxos?
    //     // else ignore
    //     // is transaction valid?
    //     // hmm does that do duplicate work? maybe instead we should immediately add it to memPool
    //     // or rather to a validation pre-filter pool?
    //     // invalid transactions cache??
    //     // if receiving an invalid transaction from a node, immediately remove that node.
    //
    //     // generate unique channel id here for collisions
    //
    //     // //https://github.com/xacrimon/dashmap/issues/78
    //     // let result = self.transactions.entry(*transaction_hash).or_insert_with(|()| TransactionInfo {
    //     //     transaction: *transaction.clone(),
    //     //     receipt_time: 0,
    //     //     observations: vec![]
    //     // });
    //
    //     // see if we're the thread that is supposed to process this transaction.
    //
    //     // Alternatively, just get a channel here, and have that channel attempt to start acquiring
    //     // stuff on the utxos?
    //     return Ok(());
    // }

    // pub fn check_transaction_known(
    //     &self,
    //     transaction_hash: &[u8; 32],
    //     transaction: &Transaction,
    // ) -> Result<(), Error> {
    //     if self.transactions.contains_key(transaction_hash) {
    //         return Err(Error::TransactionAlreadyProcessing);
    //     }
    //     return Ok(());
    // }
}
