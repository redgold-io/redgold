use std::sync::{Arc, Mutex};
use async_std::prelude::FutureExt;
use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use tokio::task::JoinHandle;
use redgold_data::add;
use redgold_schema::{EasyJson, error_info, ErrorInfoContext, RgResult};
use redgold_schema::structs::{Address, Transaction};
use crate::core::internal_message::{Channel, new_bounded_channel, RecvAsyncErrorInfo};
use crate::core::relay::Relay;

#[derive(Clone)]
enum ContractStateMessage {
    AddRequestTransaction {
        transaction: Transaction,
        response: flume::Sender<RgResult<()>>,
    },
    DeployTransaction {
        transaction: Transaction
    },
}

#[derive(Clone)]
pub struct ContractStateManagerEntry {
    channel: Channel<ContractStateMessage>,
    // join_handle: Arc<Mutex<JoinHandle<RgResult<()>>>>,
    pub ordering_instance: ContractStateOrderingInstance,
}

impl ContractStateManagerEntry {

    pub fn new(channel_message_bound: usize, address: &Address, x: &Relay) -> Self {
        let channel = new_bounded_channel::<ContractStateMessage>(channel_message_bound);
        let mut ordering_instance = ContractStateOrderingInstance {
            receiver: channel.receiver.clone(),
            address: address.clone(),
            relay: x.clone()
        };
        let mut o2 = ordering_instance.clone();
        let jh = tokio::spawn(async move {o2.message_loop().await});
        Self {
            channel,
            // join_handle: Arc::new(Mutex::new(jh)),
            ordering_instance,
        }
    }
}

#[derive(Clone)]
pub struct ContractStateOrderingInstance {
    receiver: flume::Receiver<ContractStateMessage>,
    address: Address,
    pub relay: Relay,
}

impl ContractStateOrderingInstance {


    // TODO: Need a better solution here
    pub async fn message_loop(&mut self, ) -> RgResult<()> {
        self.message_loop_inner().await.map_err(|e| panic!("{}", e.json_or()))
    }
    pub async fn message_loop_inner(&mut self) -> RgResult<()> {
        loop {
            let message = self.receiver.recv_async_err().await?;
            match message {
                ContractStateMessage::AddRequestTransaction { .. } => {}
                ContractStateMessage::DeployTransaction { .. } => {}
            }
        }
    }
}

#[derive(Clone)]
pub struct ContractStateManager {
    pub contract_state_channels: Arc<DashMap<Address, ContractStateManagerEntry>>
}

impl ContractStateManager {
    pub fn new() -> Self {
        Self {
            contract_state_channels: Arc::new(Default::default()),
        }
    }

    // pub async fn with_jh_check(csm: &ContractStateManagerEntry) -> RgResult<()> {
    //     let guard = csm.join_handle.lock()
    //         .map_err(|e| error_info(format!("Lock failure: {}", e.to_string())))?;
    //     let jh = std::mem::take(&mut *guard);
    //     if jh.is_finished() {
    //         jh.await.error_info("join error")?
    //     } else {
    //         Ok(())
    //     }
    // }

    pub fn channel(&self, address: &Address, relay: Relay) -> RgResult<ContractStateManagerEntry> {
        // TODO: Max channels exceeded, need to rebalance to drop partition distance

        let entry = self
            .contract_state_channels
            .entry(address.clone());
        let res = match entry {
            Entry::Occupied(v) => {
                let res = v.get().clone();
                // Self::with_jh_check(&res)?;
                Ok(res)
            },
            Entry::Vacant(entry) => {
                let csme = ContractStateManagerEntry::new(
                    relay.node_config.contract.contract_state_channel_bound.clone(),
                    &address,
                    &relay
                );
                // Self::with_jh_check(&csme)?;
                Ok(csme)
            }
        };
        res
    }
}