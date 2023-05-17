// Start with 1st order observations only
// eventually, to deal with avoiding resolving, do 2nd order for observations that have
// already been resolved. i.e. only known transactions.

use std::sync::Arc;
use std::time::{Duration, SystemTime};
use itertools::Itertools;

use log::info;
use metrics::increment_counter;
use prost::{DecodeError, Message as msg};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use redgold_schema::{SafeBytesAccess, SafeOption, struct_metadata_new, WithMetadataHashable};
use crate::api::rosetta::models::Error;

use crate::core::internal_message::{FutLoopPoll, PeerMessage, SendErrorInfo};
use crate::core::relay::Relay;
use crate::schema::structs::GossipObservationRequest;
use crate::schema::structs::Request;
use crate::schema::structs::{Observation, ObservationMetadata, Proof};
use crate::schema::structs::ErrorInfo;
use crate::util;
use crate::util::{current_time_millis, random_salt};
use crate::core::internal_message::RecvAsyncErrorInfo;
use crate::data::data_store::DataStore;
use crate::schema::json;

pub struct ObservationBuffer {
    data: Vec<ObservationMetadata>,
    relay: Relay,
    latest: Option<Observation>
}

impl ObservationBuffer {
    async fn run(&mut self) -> Result<(), ErrorInfo> {

        let mut interval =
            tokio::time::interval(self.relay.node_config.observation_formation_millis);
        // TODO use a select! between these two.

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.form_observation();
                },
                // TODO use fut loop thing.
                res = self.relay.observation_metadata.receiver.recv_async_err() => {
                    increment_counter!("redgold.observation.metadata.added");
                    let o = res?;
                    log::debug!(
                        "Pushing observation metadata to buffer {}",
                        json(&o.clone())?
                    );
                    self.data.push(o);
                }
            }
        }
    }

    // https://stackoverflow.com/questions/63347498/tokiospawn-borrowed-value-does-not-live-long-enough-argument-requires-tha
    pub fn new(relay: Relay, arc: Arc<Runtime>) -> JoinHandle<Result<(), ErrorInfo>>{

        let latest = arc.block_on(relay.ds.observation
            .select_latest_observation(relay.node_config.public_key())).ok().flatten();

        let mut o = Self {
            data: vec![],
            relay,
            latest
        };
        arc.spawn(async move {
            o.run().await
        })
    }

    pub fn form_observation(&mut self) -> Result<(), ErrorInfo> {
        if self.data.is_empty() {
            return Ok(());
        }
        let clone = self.data.clone();
        self.data.clear();
        let hashes = clone
            .iter()
            .map(|r| r.hash())
            .collect_vec();
        let root = util::merkle::build(hashes)?.root;
        let vec = root.safe_bytes()?;
        let parent_hash = self.latest.clone().map(|o| o.hash());
        let height = self.latest.clone().map(|o| o.height + 1).unwrap_or(0);
        let mut o = Observation {
            merkle_root: Some(vec.clone().into()),
            observations: clone,
            proof: Some(Proof::from_keypair(
                &vec,
                self.relay.node_config.wallet().active_keypair(),
            )),
            struct_metadata: struct_metadata_new(),
            hash: None,
            salt: random_salt(),
            height,
            parent_hash,
        };
        DataStore::map_err(self.relay.ds.insert_observation(o.clone(),
                                                            o.struct_metadata.safe_get()?.time as u64))?;
        o.with_hash();
        self.latest = Some(o.clone());
        // self.relay.ds.transaction_store
        let mut request = Request::empty();
        request.gossip_observation_request = Some(GossipObservationRequest {
                observation: Some(o.clone()),
        });
        let mut message = PeerMessage::empty();
        message.request = request;
        self.relay
            .peer_message_tx
            .sender
            .send_err(message)?;
        increment_counter!("redgold.observation.created");
        info!("Formed observation {}", json(&o.clone())?);
        Ok(())
    }
}

// Broken, runs forever?
//
// #[test]
// fn test_observation_buffer() {
//     let relay = Relay::default();
//     let c = relay.ds.create_all_err_info().unwrap();
//     let runtime = build_runtime(5, "test-obs");
//     ObservationBuffer::new(relay.clone(), runtime.clone());
//     relay
//         .observation_metadata
//         .sender
//         .send(ObservationMetadata::default());
//     let obs_request = relay
//         .peer_message
//         .receiver
//         .recv()
//         .unwrap()
//         .request
//         .gossip_observation_request;
//     assert!(obs_request.is_some());
//     assert_eq!(1, obs_request.unwrap().observation.observations.len());
// }
