// Start with 1st order observations only
// eventually, to deal with avoiding resolving, do 2nd order for observations that have
// already been resolved. i.e. only known transactions.

use eframe::epaint::ahash::HashMap;
use futures::TryStreamExt;
use itertools::Itertools;
use tracing::{debug, info, trace};
use metrics::{counter, gauge};
use tokio::task::JoinHandle;
// use futures::stream::StreamExt;
use tokio::time::Interval;
// Make sure to import StreamExt
use tokio_stream::StreamExt;
use tokio_stream::wrappers::IntervalStream;
use tokio_util::either::Either;
use redgold_common_no_wasm::tx_new::TransactionBuilderSupport;
use redgold_keys::proof_support::ProofSupport;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::structs::{Hash, ObservationProof, Transaction};
use redgold_schema::tx::tx_builder::TransactionBuilder;
use redgold_schema::util::merkle::build_root;

use crate::core::internal_message::SendErrorInfo;
use crate::core::relay::{ObservationMetadataInternalSigning, Relay};
use redgold_schema::helpers::easy_json::json;
use redgold_schema::observability::errors::Loggable;
use crate::node_config::WordsPassNodeConfig;
use crate::schema::structs::{Observation, ObservationMetadata};
use crate::schema::structs::ErrorInfo;
use crate::schema::structs::GossipObservationRequest;
use crate::schema::structs::Request;

const ANCESTOR_MERKLE_ROOT_LENGTH: usize = 1000;

pub struct ObservationBuffer {
    data: Vec<ObservationMetadata>,
    relay: Relay,
    latest: Transaction,
    ancestors: Vec<Hash>,
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

        let latest = if let Some(tx) =
            relay.ds.observation.select_latest_observation(relay.node_config.public_key())
                .await.expect("Error loading observation initial") {
            tx
        } else {
            let mut tx = TransactionBuilder::new(&relay.node_config);
            // relay.get_self_fee_utxo()
            tx.allow_bypass_fee = true;
            let address = relay.node_config.public_key().address().expect("address");
            tx.with_genesis_input(&address);
            tx.with_observation(&Observation::default(), 0, &address)
                .with_pow().expect("pow");
            let tx = tx.transaction.sign(&relay.node_config.words().default_kp().expect("kp")).expect("sign");
            tx
        };

        let cur_height = latest.height().expect("h");
        // TODO: Verify this calculation is correct.
        let ancestor_cutoff = cur_height - (cur_height % ANCESTOR_MERKLE_ROOT_LENGTH as i64);

        let ancestors = relay.ds.observation.recent_observation(Some(ANCESTOR_MERKLE_ROOT_LENGTH as i64))
            .await.expect("Error loading observation initial").iter()
            .filter(|t| t.height().expect("h") >= ancestor_cutoff)
            .map(|a| a.hash_or())
            .collect_vec();

        debug!("Starting observation buffer with latest observation hash: {}", latest.hash_or().hex());

        let interval1 = tokio::time::interval(relay.node_config.observation_formation_millis.clone());
        let mut o = Self {
            data: vec![],
            relay,
            latest,
            ancestors,
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
                    s.send_rg_err(o.clone()).log_error().ok();
                }
            }
        }
        Ok(())
    }

    pub async fn process_incoming(&mut self, o: ObservationMetadataInternalSigning) -> Result<(), ErrorInfo> {
        if let Some(h) = o.observation_metadata.observed_hash.clone() {
            counter!("redgold.observation.buffer.added").increment(1);
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
        counter!("redgold.observation.attempt").increment(1);

        let observations = self.data.clone();
        let num_observations = observations.len();
        self.data.clear();
        let hashes = observations
            .iter()
            .map(|r| r.hash_or())
            .collect_vec();
        let root = build_root(hashes)?.root;
        let height = self.latest.height().expect("Missing height on internal observation") + 1;
        let utxo_id = self.latest.observation_as_utxo_id()?;

        let ancestor_root = if self.ancestors.len() == ANCESTOR_MERKLE_ROOT_LENGTH {
            let tree = build_root(self.ancestors.clone())?;
            self.ancestors.clear();
            Some(tree.root)
        } else {
            None
        };

        let ancestor_roots = vec![ancestor_root].into_iter().flatten().collect_vec();

        let o = Observation {
            merkle_root: Some(root),
            observations,
            parent_id: Some(utxo_id.clone()),
            ancestor_merkle_roots: ancestor_roots,
        };
        let mut tx_b = TransactionBuilder::new(&self.relay.node_config);
        tx_b.allow_bypass_fee = true;
        let utxo_e = self.latest.observation_output_as()?;
        tx_b.with_observation(&o, height, &self.relay.node_config.address());
        tx_b.with_input(&utxo_e.to_input()).with_pow()?;
        let signed_tx = tx_b.transaction
            .sign(&self.relay.node_config.words().default_kp()?)?;


        self.relay.ds.observation.insert_observation_and_edges(&signed_tx).await?;
        // Verify stored.
        assert!(self.relay.ds.observation.query_observation(&signed_tx.hash_or()).await?.is_some());
        // TODO: Test to see if one of the transactions was stored correctly.
        self.latest = signed_tx.clone();
        self.ancestors.push(signed_tx.hash_or());

        // TODO: Use full pattern here
        // let peers = self.relay.ds.peer_store.active_nodes_ids(None).await?;
        // let trust = self.relay.get_trust().await?;

        // Send each proof individually to observed hashes of subscribers by XOR.
        // Separate distance of UTXO conflict from UTXO store?? Or is it the same?
        let proofs = o.build_observation_proofs(
            &signed_tx.hash_or(), &signed_tx.observation_proof()?.clone()
        );
        // Important, do not send to peers which have already seen the observation.

        // self.relay.ds.transaction_store
        let mut request = Request::empty();
        request.gossip_observation_request = Some(GossipObservationRequest {
                observation: Some(signed_tx.clone()),
        });
        self.relay.gossip_req(&request, &signed_tx.hash_or()).await?;
        counter!("redgold.observation.created").increment(1);
        gauge!("redgold.observation.height").set(height as f64);
        gauge!("redgold.observation.last.size").set(num_observations as f64);
        for _ in 0..num_observations {
            counter!("redgold.observation.metadata.total").increment(1);
        }
        let node_id = self.relay.node_config.short_id()?;
        trace!("node_id={} Formed observation {}", node_id, json(&o.clone())?);
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
