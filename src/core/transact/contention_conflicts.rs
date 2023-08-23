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
use redgold_schema::structs::{Address, ContentionKey, ContractStateMarker, ExecutionInput, Hash, ObservationMetadata, Output, StateSelector, Transaction, TransactionInfo};
use crate::core::internal_message::{Channel, new_bounded_channel, RecvAsyncErrorInfo, SendErrorInfo};
use crate::core::process_transaction::ProcessTransactionMessage;
use crate::core::relay::Relay;
use crate::core::stream_handlers::IntervalFoldOrReceive;
use crate::util;

#[derive(Clone)]
pub struct ContentionInfo {

}

#[derive(Clone)]
pub enum ContentionMessageInner {
    RegisterPotentialContention {
        transaction_hash: Hash,
    },
    ObservationInfo {
        observation_metadata: ObservationMetadata
    },
}

#[derive(Clone)]
pub struct ContentionMessage {
    key: ContentionKey,
    response: Sender<RgResult<ContentionInfo>>,
    message: ContentionMessageInner
}

impl ContentionMessage {
    pub fn new(key: &ContentionKey, message: ContentionMessageInner, response: Sender<RgResult<ContentionInfo>>) -> Self {
        Self {
            key: key.clone(),
            response,
            message
        }
    }
}


pub struct ContentionConflictManager {
    relay: Relay,
    unordered: HashMap<ContentionKey, Vec<(Transaction, i64)>>,
    subscribers: HashMap<ContentionKey, Vec<flume::Sender<RgResult<ContractStateMarker>>>>
}

impl ContentionConflictManager {
    pub fn new(relay: Relay) -> Self {
        Self {
            relay,
            unordered: Default::default(),
            subscribers: Default::default(),
        }
    }
    pub async fn process_tx(&mut self,
                            msg: &ContentionKey,
                            x: &ContentionMessageInner
    ) -> RgResult<ContentionInfo> {
        //
        // if output.is_deploy() {
        //     Err(error_info("Deploy transactions not supported"))?;
        // }
        // if !output.is_request() {
        //     Err(error_info("Non-request transaction outputs not supported"))?;
        // }
        // let contention_key = output.request_contention_key()?;
        // let tuple = (transaction.clone(), util::current_time_millis_i64());
        // let result = self.unordered.get_mut(&contention_key);
        // if let Some(vec) = result {
        //     vec.push(tuple);
        // } else {
        //     self.unordered.insert(contention_key.clone(), vec![tuple]);
        // }
        // if let Some(vec) = self.subscribers.get_mut(&contention_key) {
        //     vec.push(response.clone());
        // } else {
        //     self.subscribers.insert(contention_key, vec![response.clone()]);
        // }
        Ok(ContentionInfo{})
    }
    async fn interval(&mut self) -> RgResult<()> {
        Ok(())
    }

}

// TODO: Pull these out into a trait
#[async_trait]
impl IntervalFoldOrReceive<ContentionMessage> for ContentionConflictManager {
    async fn interval_fold_or_recv(&mut self, message: Either<ContentionMessage, ()>) -> RgResult<()> {
        match message {
            Either::Left(m) => {
                m.response.send_err(self.process_tx(&m.key, &m.message).await)?;
            }
            Either::Right(_) => {
                self.interval().await?;
            }
        }
        Ok(())
    }
    //

    // async fn process_message(&mut self, message: ContentionMessage) -> RgResult<()> {
    //
    // }
}