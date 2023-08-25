use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use async_std::prelude::FutureExt;
use async_trait::async_trait;
use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use flume::Sender;
use futures::future::Either;
use tokio::task::JoinHandle;
use redgold_data::add;
use redgold_schema::{EasyJson, error_info, ErrorInfoContext, RgResult, SafeBytesAccess, SafeOption, WithMetadataHashable};
use redgold_schema::structs::{Address, ContentionKey, ContractStateMarker, ExecutionInput, Output, StateSelector, Transaction};
use crate::core::internal_message::{Channel, new_bounded_channel, RecvAsyncErrorInfo, SendErrorInfo};
use crate::core::process_transaction::ProcessTransactionMessage;
use crate::core::relay::Relay;
use crate::core::stream_handlers::IntervalFoldOrReceive;
use crate::util;

//
// #[derive(Clone)]
// pub struct ContractStateManagerEntry {
//     channel: Channel<ContractStateMessage>,
//     // join_handle: Arc<Mutex<JoinHandle<RgResult<()>>>>,
//     pub ordering_instance: ContractStateOrderingInstance,
// }
//
// impl ContractStateManagerEntry {
//
//     pub fn new(channel_message_bound: usize, address: &Address, x: &Relay) -> Self {
//         let channel = new_bounded_channel::<ContractStateMessage>(channel_message_bound);
//         let mut ordering_instance = ContractStateOrderingInstance {
//             receiver: channel.receiver.clone(),
//             address: address.clone(),
//             relay: x.clone()
//         };
//         let mut o2 = ordering_instance.clone();
//         let jh = tokio::spawn(async move {o2.message_loop().await});
//         Self {
//             channel,
//             // join_handle: Arc::new(Mutex::new(jh)),
//             ordering_instance,
//         }
//     }
// }
//
// #[derive(Clone)]
// pub struct ContractStateOrderingInstance {
//     receiver: flume::Receiver<ContractStateMessage>,
//     address: Address,
//     pub relay: Relay,
// }
//
// impl ContractStateOrderingInstance {
//
//
//     // TODO: Need a better solution here
//     pub async fn message_loop(&mut self, ) -> RgResult<()> {
//         self.message_loop_inner().await.map_err(|e| panic!("{}", e.json_or()))
//     }
//     pub async fn message_loop_inner(&mut self) -> RgResult<()> {
//         loop {
//             let message = self.receiver.recv_async_err().await?;
//             match message {
//                 ContractStateMessage::AddRequestTransaction { .. } => {}
//                 ContractStateMessage::DeployTransaction { .. } => {}
//             }
//         }
//     }
// }
//
// #[derive(Clone)]
// pub struct ContractStateManager {
//     pub contract_state_channels: Arc<DashMap<Address, ContractStateManagerEntry>>
// }
//
// impl ContractStateManager {
//     pub fn new() -> Self {
//         Self {
//             contract_state_channels: Arc::new(Default::default()),
//         }
//     }
//
//     // pub async fn with_jh_check(csm: &ContractStateManagerEntry) -> RgResult<()> {
//     //     let guard = csm.join_handle.lock()
//     //         .map_err(|e| error_info(format!("Lock failure: {}", e.to_string())))?;
//     //     let jh = std::mem::take(&mut *guard);
//     //     if jh.is_finished() {
//     //         jh.await.error_info("join error")?
//     //     } else {
//     //         Ok(())
//     //     }
//     // }
//
//     pub fn channel(&self, address: &Address, relay: Relay) -> RgResult<ContractStateManagerEntry> {
//         // TODO: Max channels exceeded, need to rebalance to drop partition distance
//
//         let entry = self
//             .contract_state_channels
//             .entry(address.clone());
//         let res = match entry {
//             Entry::Occupied(v) => {
//                 let res = v.get().clone();
//                 // Self::with_jh_check(&res)?;
//                 Ok(res)
//             },
//             Entry::Vacant(entry) => {
//                 let csme = ContractStateManagerEntry::new(
//                     relay.node_config.contract.contract_state_channel_bound.clone(),
//                     &address,
//                     &relay
//                 );
//                 // Self::with_jh_check(&csme)?;
//                 Ok(csme)
//             }
//         };
//         res
//     }
// }


#[derive(Clone)]
pub enum ContractStateMessage {
    ProcessTransaction {
        transaction: Transaction,
        output: Output,
        response: flume::Sender<RgResult<ContractStateMarker>>,
    },
}


pub struct ContractStateManager {
    relay: Relay,
    unordered: HashMap<ContentionKey, Vec<(Transaction, i64)>>,
    subscribers: HashMap<ContentionKey, Vec<flume::Sender<RgResult<ContractStateMarker>>>>
}

#[derive(Clone)]
struct UnorderedContractUpdate {
    transaction: Transaction,
    output: Output,
    receipt_time: i64
}

impl ContractStateManager {
    pub fn new(relay: Relay) -> Self {
        Self {
            relay,
            unordered: Default::default(),
            subscribers: Default::default(),
        }
    }
    pub async fn process_tx(&mut self,
                            transaction: &Transaction,
                            output: &Output,
                            response: &Sender<RgResult<ContractStateMarker>>
    ) -> RgResult<()> {

        if output.is_deploy() {
            Err(error_info("Deploy transactions not supported"))?;
        }
        if !output.is_request() {
            Err(error_info("Non-request transaction outputs not supported"))?;
        }
        let contention_key = output.request_contention_key()?;
        let tuple = (transaction.clone(), util::current_time_millis_i64());
        let result = self.unordered.get_mut(&contention_key);
        if let Some(vec) = result {
            vec.push(tuple);
        } else {
            self.unordered.insert(contention_key.clone(), vec![tuple]);
        }
        if let Some(vec) = self.subscribers.get_mut(&contention_key) {
            vec.push(response.clone());
        } else {
            self.subscribers.insert(contention_key, vec![response.clone()]);
        }
        Ok(())
    }

    // TODO: Introduce time ordering strategy selector, additional proofs, etc.
    pub async fn interval(&mut self) -> RgResult<()> {
        let ct = util::current_time_millis_i64();
        let mut finished_contention_keys = vec![];
        for (k, v) in self.unordered.iter_mut() {
            // TODO: Earlier validation should find something in the recent state -- at least
            // something close-ish since none of these transactions can be guaranteed to be issuing a
            // recent reference due to contention time.
            let address = k.address.safe_get_msg("Missing address on contention key")?;
            let sel = k.selector.as_ref();
            let most_recent_query_result = self.relay.ds.state.query_recent_state(
                address,
                sel,
                Some(1)
            ).await?;
            let head = most_recent_query_result.get(0);
            let most_recent = head.safe_get_msg("Missing most recent contract state")?;
            let resolve = self.relay.ds.resolve_code(address).await?;
            let u = resolve.utxo_entry.safe_get_msg("Code Utxo")?;
            let o = u.output.safe_get_msg("Output")?;
            let code_opt = o.code();
            let code = code_opt.safe_get_msg("Code")?;

            // TODO: This is only one strategy, and missing proof steps etc.
            let mut v2 = v.clone();
            v2.sort_by(|a, b| a.0.time().expect("t").cmp(&b.0.time().expect("t")));
            for (idx, (tx, t)) in v2.iter().enumerate() {
                // TODO: Send to executor channel rather than processing here locally.
                let output_col = tx.output_of(&address);
                let output_of = output_col.get(0);
                let output = output_of.safe_get_msg("Missing output")?;
                let input = output.request_data()?;
                let er = redgold_executor::extism_wrapper::invoke_extism_wasm_direct(
                    code,
                    input,
                    most_recent.state.safe_bytes()?.as_ref(),
                ).await?;
                let d = er.data.safe_get_msg("data")?;
                let updated_state = d.state.safe_get_msg("state")?;
                let mut csm = ContractStateMarker::default();
                csm.state = Some(updated_state.clone());
                csm.address = Some(address.clone());
                csm.selector = sel.cloned();
                csm.time = tx.time()?.clone();
                csm.transaction_marker = Some(tx.hash_or());
                csm.nonce = most_recent.nonce + 1;
                self.relay.ds.state.insert_state(csm.clone()).await?;
                if let Some(subs) = self.subscribers.remove(k) {
                    for sub in &subs {
                        sub.send_err(Ok(csm.clone()))?;
                    }
                }
                v.remove(idx);
            }
            finished_contention_keys.push(k.clone())
        }
        for k in &finished_contention_keys {
            self.unordered.remove(k);
        }
        Ok(())
    }
}
#[async_trait]
impl IntervalFoldOrReceive<ContractStateMessage> for ContractStateManager {
    async fn interval_fold_or_recv(&mut self, message: Either<ContractStateMessage, ()>) -> RgResult<()> {
        match message {
            Either::Left(m) => {
                match m {
                    ContractStateMessage::ProcessTransaction { transaction, output, response } => {
                        if let Err(e) = self.process_tx(&transaction, &output, &response).await {
                            response.send_err(Err(e))?;
                        }
                    }
                }
            }
            Either::Right(_) => {
                self.interval().await?;
            }
        }
        Ok(())
    }
}