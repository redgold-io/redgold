// Start with 1st order observations only
// eventually, to deal with avoiding resolving, do 2nd order for observations that have
// already been resolved. i.e. only known transactions.

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_std::prelude::FutureExt;
use eframe::epaint::ahash::HashMap;
use futures::TryStreamExt;
use itertools::Itertools;
use log::info;
use metrics::{gauge, increment_counter, increment_gauge};
use prost::{DecodeError, Message as msg};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
// use futures::stream::StreamExt;
use tokio::time::Interval;
// Make sure to import StreamExt
use tokio_stream::StreamExt;
use tokio_stream::wrappers::IntervalStream;
use tokio_util::either::Either;

use redgold_schema::{SafeBytesAccess, SafeOption, struct_metadata_new, WithMetadataHashable};
use redgold_schema::EasyJson;
use redgold_schema::structs::{Hash, ObservationProof};

use crate::api::rosetta::models::Error;
use crate::core::internal_message::{PeerMessage, SendErrorInfo};
use crate::core::internal_message::RecvAsyncErrorInfo;
use crate::core::relay::{ObservationMetadataInternalSigning, Relay};
use redgold_data::data_store::DataStore;
use crate::schema::json;
use crate::schema::json_or;
use crate::schema::structs::{Observation, ObservationMetadata, Proof};
use crate::schema::structs::ErrorInfo;
use crate::schema::structs::GossipObservationRequest;
use crate::schema::structs::Request;
use crate::util;
use crate::util::{current_time_millis, random_salt};

pub struct ObservationBuffer {
    data: Vec<ObservationMetadata>,
    relay: Relay,
    latest: Option<Observation>,
    subscribers: HashMap<Hash, flume::Sender<ObservationProof>>,
    interval: Interval
}

impl ObservationBuffer {

    async fn handle_message(&mut self, o: Either<ObservationMetadataInternalSigning, ()>) -> Result<&mut Self, ErrorInfo> {
        match o {
            Either::Left(o) => {
                self.process_incoming(o).await?;
            }
            Either::Right(_) => {
                self.form_and_respond().await?;
            }
        }
        Ok(self)
    }

    async fn run(&mut self) -> Result<(), ErrorInfo> {

        let interval =
            tokio::time::interval(self.relay.node_config.observation_formation_millis);

        let interval_stream = IntervalStream::new(interval).map(|_| Ok(Either::Right(())));


        let recv = self.relay.observation_metadata.receiver.clone();
        let stream = recv
            .into_stream()
            .map(|x| Ok(Either::Left(x)));

        stream.merge(interval_stream).try_fold(
            Ok(self), |ob, o| async {
                let ress = match ob {
                    Ok(ooo) => {
                        let res = ooo.handle_message(o).await.map(|x| Ok(x));
                        res
                    }
                    Err(e) => {
                        let res: Result<Result<&mut Self, _>, ErrorInfo> = Err(e);
                        res
                    }
                };
                ress
            }
        ).await??;

    // }
        //
        // loop {
        //     tokio::select! {
        //         _ = interval.tick() => {
        //             match self.form_and_respond().await {
        //                 Ok(_) => {},
        //                 Err(e) => {
        //                     log::error!("Error forming observation: {}", e.json_or());
        //                 }
        //             }
        //         },
        //         // TODO use fut loop thing.
        //         res = self.relay.observation_metadata.receiver.recv_async_err() => {
        //             let o: ObservationMetadataInternalSigning = res?;
        //             self.process_incoming(o).await;
        //         }
        //     }
        // }
        Ok(())
    }

    // https://stackoverflow.com/questions/63347498/tokiospawn-borrowed-value-does-not-live-long-enough-argument-requires-tha
    pub async fn new(relay: Relay,
               // arc: Arc<Runtime>
    ) -> JoinHandle<Result<(), ErrorInfo>>{

        let latest =
            // arc.block_on(
            relay.ds.observation
            .select_latest_observation(relay.node_config.public_key())
                .await.ok().flatten();

        info!("Starting observation buffer with latest observation: {}", latest.json_or());

        let interval1 = tokio::time::interval(relay.node_config.observation_formation_millis.clone());
        let mut o = Self {
            data: vec![],
            relay,
            latest,
            subscribers: Default::default(),
            interval: interval1
        };
        tokio::spawn(async move {
            o.run().await
        })
    }

    pub async fn form_and_respond(&mut self) -> Result<(), ErrorInfo> {
        let proofs = self.form_observation().await?;
        for o in proofs {
            if let Some(oh) = o.metadata.clone().and_then(|m| m.observed_hash) {
                if let Some(s) = self.subscribers.get(&oh) {
                    // info!("Responding to sender with observation proof");
                    s.send_err(o.clone())?;
                }
            }
        }
        Ok(())
    }

    pub async fn process_incoming(&mut self, o: ObservationMetadataInternalSigning) -> Result<(), ErrorInfo> {
        if let Some(h) = o.observation_metadata.observed_hash.clone() {
            increment_counter!("redgold.observation.buffer.added");
            // log::info!("Pushing observation metadata to buffer {}", json_or(&o.observation_metadata.clone()));
            self.subscribers.insert(h.clone(), o.sender.clone());
            self.data.push(o.observation_metadata);
        };
        Ok(())
    }

    pub async fn form_observation(&mut self) -> Result<Vec<ObservationProof>, ErrorInfo> {
        if self.data.is_empty() {
            return Ok(vec![]);
        }
        // info!("Forming observation");
        increment_counter!("redgold.observation.attempt");

        let clone = self.data.clone();
        let num_observations = clone.len();
        self.data.clear();
        let hashes = clone
            .iter()
            .map(|r| r.hash_or())
            .collect_vec();
        let root = redgold_schema::util::merkle::build_root(hashes)?.root;
        let vec = root.safe_bytes()?;
        let parent_hash = self.latest.clone().map(|o| o.hash_or());
        let height = self.latest.clone().map(|o| o.height + 1).unwrap_or(0);
        let struct_metadata = struct_metadata_new();
        let mut o = Observation {
            merkle_root: Some(vec.clone().into()),
            observations: clone,
            proof: Some(Proof::from_keypair(
                &vec,
                self.relay.node_config.internal_mnemonic().active_keypair(),
            )),
            struct_metadata: struct_metadata.clone(),
            salt: random_salt(),
            height,
            parent_hash,
        };
        o.with_hash();
        let proofs = o.build_observation_proofs();
        self.relay.ds.observation.insert_observation_and_edges(&o, struct_metadata.safe_get()?.time.expect("time")).await?;

        // Verify stored.
        assert!(self.relay.ds.observation.query_observation(&o.hash_or()).await?.is_some());

        self.latest = Some(o.clone());
        // self.relay.ds.transaction_store
        let mut request = Request::empty();
        request.gossip_observation_request = Some(GossipObservationRequest {
                observation: Some(o.clone()),
        });
        self.relay.gossip_req(&request, &o.hash_or()).await?;
        increment_counter!("redgold.observation.created");
        gauge!("redgold.observation.height", height as f64);
        gauge!("redgold.observation.last.size", num_observations as f64);
        for _ in 0..num_observations {
            increment_counter!("redgold.observation.metadata.total");
        }
        let node_id = self.relay.node_config.short_id()?;
        info!("node_id={} Formed observation {}", node_id, json(&o.clone())?);
        Ok(proofs)
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
