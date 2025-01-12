use std::collections::HashMap;
// use async_std::prelude::FutureExt;
use async_trait::async_trait;
use flume::Sender;
use futures::future::Either;
use redgold_schema::{error_info, ErrorInfoContext, RgResult};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::structs::{ContentionKey, Hash, ObservationMetadata, ObservationProof};
use redgold_common::flume_send_help::SendErrorInfo;
use crate::core::relay::Relay;
use redgold_common_no_wasm::stream_handlers::IntervalFoldOrReceive;
use crate::util;

#[derive(Clone)]
pub struct ContentionResult {
    pub winner: Option<Hash>,
    pub no_contest: bool,
    pub proofs: Vec<ObservationProof>
}

#[derive(Clone)]
pub enum ContentionMessageInner {
    RegisterPotentialContention {
        transaction_hash: Hash,
    },
    ObservationInfo {
        observation_metadata: ObservationMetadata
    },
    CheckContentionAccepted {
        transaction_hash: Hash
    }
}

#[derive(Clone)]
pub struct ContentionMessage {
    key: ContentionKey,
    response: Sender<RgResult<ContentionResult>>,
    message: ContentionMessageInner
}

impl ContentionMessage {
    pub fn new(key: &ContentionKey, message: ContentionMessageInner, response: Sender<RgResult<ContentionResult>>) -> Self {
        Self {
            key: key.clone(),
            response,
            message
        }
    }
}


pub struct ContentionConflictManager {
    relay: Relay,
    contentions: HashMap<ContentionKey, HashMap<Hash, i64>>,
    subscribers: HashMap<ContentionKey, Vec<Sender<RgResult<ContentionResult>>>>,
    // Consider using this later to store proofs? Or just use internal ds for now
    proof_buffer: HashMap<Hash, Vec<ObservationProof>>
}

impl ContentionConflictManager {
    pub fn new(relay: Relay) -> Self {
        Self {
            relay,
            contentions: Default::default(),
            subscribers: Default::default(),
            proof_buffer: Default::default(),
        }
    }
    pub async fn process_message(&mut self,
                                 key: &ContentionKey,
                                 msg: &ContentionMessageInner,
                                 _response: &Sender<RgResult<ContentionResult>>
    ) -> RgResult<ContentionResult> {
        let time = util::current_time_millis_i64();
        match msg {
            ContentionMessageInner::RegisterPotentialContention { transaction_hash: hash } => {
                if let Some(contentions) = self.contentions.get_mut(key) {
                    if let Some(ts) = contentions.get(hash) {
                        // Some other transaction thread is already processing this hash,
                        // this shouldn't happen since there's a check in transaction processing already
                        return Err(error_info(
                            format!("Duplicate contention, hash already registered at time {} {}", ts, hash.json_or())
                        ));
                    } else {
                        contentions.insert(hash.clone(), time);
                        return Ok(ContentionResult {
                            winner: Some(hash.clone()),
                            no_contest: true,
                            proofs: vec![],
                        })
                    }
                } else {
                    self.contentions.insert(key.clone(), HashMap::from([(hash.clone(), util::current_time_millis_i64())]));
                }
            }
            // Not used yet
            ContentionMessageInner::ObservationInfo { .. } => {

            }
            ContentionMessageInner::CheckContentionAccepted { transaction_hash: _ } => {}
        }
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
        Ok(ContentionResult {
            winner: None,
            no_contest: false,
            proofs: vec![],
        })
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
                m.response.send_rg_err(self.process_message(&m.key, &m.message, &m.response).await)?;
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