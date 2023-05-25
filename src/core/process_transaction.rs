use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crossbeam_channel::{unbounded, Receiver, Sender};
use dashmap::mapref::entry::Entry;
use futures::{TryFutureExt, TryStreamExt};
use itertools::Itertools;
use log::{debug, error, info};
use metrics::increment_counter;
use tokio::runtime::Runtime;
use tokio::select;
use tokio::task::{JoinError, JoinHandle};
use uuid::Uuid;
use redgold_schema::{json_or, ProtoHashable, SafeOption, struct_metadata_new, structs, task_local, task_local_map};
use redgold_schema::structs::{FixedUtxoId, GossipTransactionRequest, Hash, PublicResponse, QueryObservationProofRequest, Request, ValidationType};

use crate::core::internal_message::{FutLoopPoll, PeerMessage, RecvAsyncErrorInfo, SendErrorInfo, TransactionMessage};
use crate::core::relay::Relay;
use crate::core::transaction::{TransactionTestContext, validate_utxo};
use crate::data::data_store::DataStore;
use crate::schema::structs::{Error, ResponseMetadata};
use crate::schema::structs::{HashType, ObservationMetadata, State, Transaction};
use crate::schema::structs::{QueryTransactionResponse, SubmitTransactionResponse};
use crate::schema::{SafeBytesAccess, WithMetadataHashable};
use crate::util::runtimes::build_runtime;
// TODO config
use crate::schema::structs::ErrorInfo;
use crate::schema::structs::ObservationProof;
use crate::schema::{empty_public_response, error_info, error_message};
use crate::util;
use futures::{stream::FuturesUnordered, StreamExt};
use crate::core::resolver::resolve_transaction;
use crate::core::transact::utxo_conflict_resolver::check_utxo_conflicts;
use crate::util::current_time_millis_i64;

#[derive(Clone)]
pub struct Conflict {
    //  Not really necessary but put other info here.
    transaction_hash: Hash,
    abort: bool,
    processing_start_time: i64,
    request_processer: RequestProcessor,
}

#[derive(Clone)]
pub struct RequestProcessor {
    sender: flume::Sender<Conflict>,
    receiver: flume::Receiver<Conflict>,
    request_id: String,
    pub transaction_hash: Hash,
}

#[derive(Clone)]
pub struct UTXOContentionPool {
    active_requests: Vec<Conflict>,
}

impl RequestProcessor {
    fn new(transaction_hash: &Hash, request_id: String) -> RequestProcessor {
        let (s, r) = flume::unbounded::<Conflict>();
        return RequestProcessor {
            sender: s,
            receiver: r,
            request_id,
            transaction_hash: transaction_hash.clone(),
        };
    }
}

#[derive(Clone)]
pub struct TransactionProcessContext {
    relay: Relay,
    tx_process: Arc<Runtime>,
    request_processor: Option<RequestProcessor>,
    transaction_hash: Option<Hash>,
    utxo_ids: Option<Vec<FixedUtxoId>>
}

// async fn query_self_accepted_transaction_status(
//     relay: Relay,
//     transaction_hash: &Hash,
// ) -> Result<QueryTransactionResponse, ErrorInfo> {
//     //relay.ds.select_peer_trust()
//     // TODO: Add hasher interface to relay / node_config
//     // let leaf_hash = util::dhash_vec(&transaction_hash).to_vec();
//
//     // TODO: add fields pct_acceptance network 0.8 for instance fraction of peers / fraction of trust accepted.
//     // TODO: Trust within partition id. partitions should be flexible in size relative to node size.
//     // or rather node size determines minimum -- partition?
//
//     let peer_publics = DataStore::map_err(relay.ds.select_broadcast_peers())?;
//
//     let mut pk = peer_publics
//         .iter()
//         .map(|x| x.public_key.clone())
//         .collect::<HashSet<Vec<u8>>>();
//
//     pk.insert(relay.node_config.wallet().transport_key().public_key_vec());
//
//     if pk.is_empty() {
//         error!("Peer keys empty, unable to validate transaction!")
//     }
//
//     let mut interval =
//         tokio::time::interval(relay.node_config.check_observations_done_poll_interval);
//
//     let mut res: Vec<ObservationProof> = vec![];
//     // what we should do is instead, in the process thing, cause a delay if we don't get any
//     // new information.
//
//     for _ in 0..relay.node_config.check_observations_done_poll_attempts {
//         interval.tick().await;
//         res = relay
//             .ds
//
//             .query_observation_edge(transaction_hash.vec())
//             .unwrap();
//         info!(
//             "Queried {:?} merkle proofs for transaction_hash {}",
//             res.len(),
//             transaction_hash.hex()
//         );
//         // TODO: Change to trust threshold for acceptance.
//         if res.len() >= 1 {
//             break;
//         }
//     }
//     Ok(QueryTransactionResponse {
//         observation_proofs: res,
//         block_hash: None,
//     })
// }

async fn resolve_conflict(relay: Relay, conflicts: Vec<Conflict>) -> Result<Hash, ErrorInfo> {
    //relay.ds.select_peer_trust()
    // TODO: Add hasher interface to relay / node_config
    // let leaf_hash = util::dhash_vec(&transaction_hash).to_vec();

    // TODO: add fields pct_acceptance network 0.8 for instance fraction of peers / fraction of trust accepted.
    // TODO: Trust within partition id. partitions should be flexible in size relative to node size.
    // or rather node size determines minimum -- partition?

    let peer_publics = relay.ds.select_broadcast_peers().unwrap();
    let mut map = HashMap::<Vec<u8>, f64>::new();
    for peer in peer_publics {
        map.insert(peer.public_key, peer.trust);
    }

    let mut trust_conflicts = vec![];
    for c in conflicts {
        let res = relay
            .ds
            .observation.select_observation_edge(&c.transaction_hash.clone())
            .await?;
        // TODO: .toMap trait.
        // TODO: group by peer id instead, more accurate
        let sum_trust = res
            .iter()
            // TODO: drop functional syntax here or deal with error somehow
            .into_group_map_by(|x| {
                let ret: Vec<u8> = x
                    .proof
                    .as_ref()
                    .map(|p| p.public_key_bytes().as_ref().unwrap().clone())
                    .ok_or("")
                    .unwrap();
                ret
            })
            .iter()
            .map(|(k, _)| map.get(k).unwrap_or(&(0 as f64)))
            .sum::<f64>();
        trust_conflicts.push((
            (-1 as f64) * sum_trust,
            c.processing_start_time,
            c.transaction_hash.clone(),
        ));
    }
    // TODO: Order hash directly in proto schema generation
    trust_conflicts.sort_by(|(_, _, h1), (_, _, h2)| h1.vec().cmp(&h2.vec()));
    Ok(trust_conflicts.get(0).unwrap().2.clone())
}

impl TransactionProcessContext {

    // Init
    pub fn new(relay: Relay, tx_process_listener: Arc<Runtime>, tx_process: Arc<Runtime>) -> JoinHandle<Result<(), ErrorInfo>> {
        let context = Self {
            relay,
            tx_process: tx_process.clone(),
            request_processor: None,
            transaction_hash: None,
            utxo_ids: None,
        };

        return tx_process_listener.spawn(async move { context.run().await });
    }

    /// Loop to check messages and process them
    // TODO: Abstract this out
    async fn run(&self) -> Result<(), ErrorInfo> {
        increment_counter!("redgold.node.async_started");
        let mut fut = FutLoopPoll::new();
        // TODO: Change to queue
        let mut receiver = self.relay.transaction.receiver.clone();
        // TODO this accomplishes queue, we can also move the spawn function into FutLoopPoll so that
        // the only function we have here is to call one function
        // do that later
        // receiver.stream().try_for_each_concurrent()
        // We can also use a Context type parameter over it to keep track of some internal context
        // per request i.e. RequestContext.
        fut.run_fut(&|| receiver.recv_async_err(), |transaction_res: Result<TransactionMessage, ErrorInfo>| {
            let mut x = self.clone();
            let jh = self.tx_process.clone()
            .spawn(async move {
                match transaction_res {
                    Ok(transaction) => {
                        x.scoped_process_and_respond(transaction).await
                    }
                    Err(e) => Err(e)
                }
            });
            jh
        } ).await
    }

    async fn scoped_process_and_respond(&mut self, transaction_message: TransactionMessage) -> Result<(), ErrorInfo> {
        let request_uuid = Uuid::new_v4().to_string();
        let hex = transaction_message.transaction.calculate_hash().hex();
        let time = transaction_message.transaction.struct_metadata.clone().map(|s| s.time.clone()).unwrap_or(0);
        let current_time = util::current_time_millis_i64();
        let input_address = transaction_message.transaction.first_input_address().clone()
            .and_then(|a| a.render_string().ok()).unwrap_or("".to_string());
        let output_address = transaction_message.transaction.first_output_address()
            .and_then(|a| a.render_string().ok()).unwrap_or("".to_string());
        let mut hm = HashMap::new();
        hm.insert("request_uuid".to_string(), request_uuid.clone());
        hm.insert("transaction_hash".to_string(), hex.clone());
        hm.insert("transaction_time".to_string(), time.to_string());
        hm.insert("current_time".to_string(), current_time.to_string());
        hm.insert("input_address".to_string(), input_address.clone());
        hm.insert("output_address".to_string(), output_address.clone());

        let res = task_local_map(hm, async move {
            self.process_and_respond(
                transaction_message, request_uuid, hex,
                time, current_time, input_address, output_address
            ).await
        }).await;
        res
    }

    #[tracing::instrument(skip(self, transaction_message))]
    async fn process_and_respond(
        &mut self,
        transaction_message: TransactionMessage,
        request_uuid: String,
        transaction_hash: String,
        transaction_time: i64,
        current_time: i64,
        input_address: String,
        output_address: String
    ) -> Result<(), ErrorInfo> {
        let result_or_error =
            self.process(&transaction_message.transaction.clone(), current_time, request_uuid).await;
        self.cleanup(None)?;

        // Use this as whether or not the request was successful
        let mut metadata = ResponseMetadata::default();
        // Change these to raw Response instead of public response
        let mut pr = structs::Response::default();
        match result_or_error {
            Ok(o) => {
                metadata.success = true;
                pr.submit_transaction_response = Some(o);
            }
            Err(ee) => {
                metadata.success = false;
                metadata.error_info = Some(ee);
            }
        }
        pr.response_metadata = Some(metadata);

        match transaction_message.response_channel {
            None => {
                increment_counter!("redgold.transaction.missing_response_channel");
                let details = ErrorInfo::error_info("Missing response channel for transaction");
                error!("Missing response channel for transaction {:?}", json_or(&details));
            }
            Some(r) => if let Some(e) = r.send_err(pr).err() {
                error!("Error sending transaction response to channel {}", json_or(&e));
            }
        };
        Ok(())
    }


    // async fn post_process_query(
    //     &self,
    //     transaction_message: Transaction,
    //     current_time: i64,
    // ) -> Result<SubmitTransactionResponse, ErrorInfo> {
    //     let result1 = tokio::time::timeout(
    //         Duration::from_secs(30),
    //         self.process(transaction.clone()),
    //     )
    //     .await
    //     .map_err(|e| error_info("timeout on transaction process"))?
    //     //.map_err(|e| error_message(e, "transaction process"))?;
    //     ?;
    //     let vec1 = transaction_message.transaction.hash_bytes()?;
    //     let status = tokio::time::timeout(
    //         Duration::from_secs(15),
    //         query_self_accepted_transaction_status(self.relay.clone(), vec1.clone()),
    //     )
    //     .await
    //     .map_err(|e| error_info("timeout on query transaction accepted status"))??;
    //     Ok(SubmitTransactionResponse {
    //         transaction_hash: transaction_message.transaction.hash().into(),
    //         query_transaction_response: Some(status),
    //         transaction: None,
    //     })
    // }

    // TODO: okay so in this loop below thre's a time which should also check if all
    // available peers have heard about a double spend etc. and potentially terminate quicker?
    // maybe? or not hell not really necessary.

    fn cleanup(&mut self, ii: Option<usize>) -> Result<(), ErrorInfo> {
        if let Some(request_processor) = &self.request_processor {
            if let Some(utxo_ids) = &self.utxo_ids {
                self.clean_utxo(&request_processor, utxo_ids, ii);
                self.relay.transaction_channels.remove(self.transaction_hash.safe_get()?);
            }
        }
        Ok(())
    }

    async fn observe(&self, validation_type: ValidationType, state: State) -> Result<ObservationProof, ErrorInfo> {
        let mut hash: Hash = self.transaction_hash.safe_get()?.clone();
        hash.hash_type = HashType::Transaction as i32;
        let mut om = structs::ObservationMetadata::default();
        om.observed_hash = Some(hash);
        om.state = Some(state as i32);
        om.struct_metadata = struct_metadata_new();
        om.observation_type = validation_type as i32;
        // TODO: It might be nice to grab the proof of a signature here?
        self.relay.observe(om).await
    }

    // TODO: Add a debug info thing here? to include data about debug calls? Thread local info? something ?
    async fn process(&mut self, transaction: &Transaction, processing_time_start: i64, request_uuid: String) -> Result<SubmitTransactionResponse, ErrorInfo> {
        increment_counter!("redgold.transaction.received");
        let hash = transaction.hash();
        self.transaction_hash = Some(hash.clone());

        /// Check if we already have a rejection reason for this transaction and abort if so
        /// returning the previous rejection reason.
        let ds = self.relay.ds.clone();
        if let Some((_, Some(pre_rejection))) = ds.transaction_store.query_maybe_transaction(&hash).await? {
            return Err(pre_rejection);
        }

        /// Establish channels for other transaction threads to communicate conflicts with this one.
        let request_processor = self.create_receiver_or_err(&hash, request_uuid)?;

        /// Validate obvious schema related errors / local errors requiring no other context information
        transaction.prevalidate()?;

        /// Attempt to resolve all the transaction inputs and outputs for context-aware validation
        /// This is the place where balances checks and signature verifications are performed.
        let resolver_data = resolve_transaction(&transaction,
                                                self.relay.clone(), self.tx_process.clone()).await?;
        resolver_data.validate()?;

        let fixed_utxo_ids = transaction.fixed_utxo_ids_of_inputs()?;
        self.utxo_ids = Some(fixed_utxo_ids.clone());
        // TODO: Check for conflicts via peer query -- currently unimplemented
        check_utxo_conflicts(self.relay.clone(), &fixed_utxo_ids, &hash).await?;

        let utxo_id_inputs = transaction.utxo_ids_of_inputs()?;

        let utxo_ids = transaction.iter_utxo_inputs();
        // let utxo_ids = validate_utxo(transaction, &self.relay.ds)?;
        let utxo_ids2 = utxo_ids.clone();
        let mut i: usize = 0;


        let mut conflict_detected = false;
        let self_conflict = Conflict {
            transaction_hash: hash.clone(),
            abort: false,
            processing_start_time: processing_time_start.clone(),
            request_processer: request_processor.clone(),
        };
        let mut conflicts: Vec<Conflict> = vec![];

        // TODO: Change this so the UTXO pool is responsible for this, remove the request processor from here
        // And create a spawned thread for each UTXO pool to handle this that returns any conflicts back here.
        for utxo_id in fixed_utxo_ids {
            // Acquire locks to determine if another transaction is re-using some output
            let entry = self.relay.utxo_channels.entry(utxo_id);
            match entry {
                Entry::Occupied(mut entry) => {
                    conflict_detected = true;
                    // issue! another pending transaction is using this
                    // this indicates a double spend
                    //receiver.
                    for active_request in entry.get().active_requests.iter() {
                        // TODO: Handle unwrap issues
                        conflicts.push(active_request.clone());
                        info!("Conflict between current: {} and {}", hash.hex(), active_request.transaction_hash.hex());
                        // Need to capture this conflict LOCALLY here for use later.
                        active_request
                            .request_processer
                            .sender
                            .send_err(self_conflict.clone())?
                    }
                    entry.get_mut().active_requests.push(self_conflict.clone());
                }
                Entry::Vacant(entry) => {
                    entry.insert(UTXOContentionPool {
                        active_requests: vec![self_conflict.clone()],
                    });
                    // Re-validate after acquiring lock. Alternative?
                    // TODO: remove this clone
                }
            };
            i += 1;
        }

        // TODO: Don't remember the purpose of this duplicate validation here, we should really
        // seed this with existing data to reduce request size and just verify it's still valid
        let resolver_data = resolve_transaction(&transaction,
                                                self.relay.clone(), self.tx_process.clone()).await?;
        resolver_data.validate()?;
        // Change this to 'revalidate' and only issue some queries again not all of them.

        let validation_type = if resolver_data.resolved_internally {
            ValidationType::Full
        } else {
            ValidationType::Partial
        };

        let mut observation_proofs = HashSet::new();

        if !conflict_detected {
            tracing::info!("Signing pending transaction");
            let prf = self.observe(validation_type, State::Pending).await?;
            observation_proofs.insert(prf);
            // TODO: Await the initial observations here, gossip about them as well?
        } else {
            tracing::info!("Conflict detected on current {}", hash.hex());
        }

        tracing::info!("Pending transaction stage started");

        // TODO: Broadcast with no concern about response, or do we check for conflicts here?
        // TODO: we really need here a mechanism to deal with incoming conflict notifications
        // A request type to handle that that'll alert this thread? OR should that just be the same
        // As transaction request? We might benefit from including additional information.
        let mut request = Request::default();
        let mut gossip_transaction_request = GossipTransactionRequest::default();
        gossip_transaction_request.transaction = Some(transaction.clone());
        request.gossip_transaction_request = Some(gossip_transaction_request);

        let mut message = PeerMessage::empty();
        message.request = request;
        self.relay
            .peer_message_tx
            .sender
            .send_err(message)?;

        tracing::info!("Gossiped transaction");

        let elapsed_time = || {
            current_time_millis_i64() - processing_time_start
        };

        // conflicts.push(watch2.unwrap());
        // let mut conflicts = vec![watch2.unwrap()];
        // Potentially receive abort code here to terminate thread early.
        // if another transaction was finalized.

        // TODO: This should really be a select between polling peers to find out if we're finished
        // And the maximum elapsed time before acceptance.
        // I.e. poll early acceptance, for now that doesn't really matter as much
        // What we should really do is notify the request processor that there is new data available
        // I.e. if some data comes in indicative of a proof or whatever, we should recv a message here
        // and process it, by adding up the observation proofs and checking if we're done.
        while elapsed_time() < (self.relay.node_config.transaction_finalization_time.as_millis() as i64) {
            let res = tokio::time::timeout(
                Duration::from_secs(1), request_processor.receiver.recv_async_err())
                .await.ok();
            if let Some(o) = res {
                let conflict = o?;
                if conflict.abort {
                    // translate error codes for db access / etc. into internal server error.
                    // TODO: This error code just indicates a conflict, not necessarily a deliberate double spend
                    // LiveConflictDetected to distinguish it.
                    return Err(error_message(Error::TransactionRejectedDoubleSpend, "Double spend detected in live context"));
                }
                conflicts.push(conflict);
            }
        }
        // TODO: we shouldn't be attempting to deal with this thread as a conflict
        /*
        Rather, what should be happening is, if we see a conflict, then we should allow that
        conflict to essentially 'take over' this thread and the other transaction threads
        and then resolve itself in ONE place, and then respond to all the response channels?
        Yeah that.
         */

        // let this_tx_trust = self.mem_pool.total_trust(&hash, &self.ds).unwrap_or(0f64);

        // TODO: Return all conflicts if not chosen as part of submit response
        // And/or return all conflicts anyways as part of the response.

        tracing::info!("Conflict resolution stage started with {:?} conflicts", conflicts.len());

        if !conflicts.is_empty() {

            let this_as_conflict = Conflict {
                transaction_hash: hash.clone(),
                abort: true,
                processing_start_time: processing_time_start.clone(),
                request_processer: request_processor.clone(),
            };
            let mut all_conflicts = conflicts.clone();
            all_conflicts.push(this_as_conflict.clone());

            let winner = resolve_conflict(self.relay.clone(), all_conflicts)
                .await?;

            if winner != hash {
                Err(error_message(Error::TransactionRejectedDoubleSpend,
                                  format!("Lost conflict to other transaction winner: {}", winner.hex())))?;
            } else {
                for c in &conflicts {
                    c.request_processer
                        .sender
                        .send_err(this_as_conflict.clone())?;
                }
            }
        }

        tracing::info!("Finalizing transaction stage started");

        let prf = self.observe(validation_type, State::Finalized).await?;
        observation_proofs.insert(prf);

        self.insert_transaction(&hash, transaction).await?;
        increment_counter!("redgold.transaction.accepted");

        tracing::info!("Finalize end on current {} with num conflicts {:?}", hash.hex(), conflicts.len());

        // Await until it has appeared in an observation and other nodes observations.

        // TODO: periodic process to clean mempool in event of thread processing crash?
        let peers = self.relay.ds.peer_store.active_nodes(None).await?;
        let mut obs_proof_req = Request::default();
        obs_proof_req.query_observation_proof_request = Some(QueryObservationProofRequest {
            hash: Some(hash.clone())
        });

        tracing::info!("Collecting observation proofs from {} peers", peers.len());
        let results = Relay::broadcast(self.relay.clone(),
                                       peers, obs_proof_req, self.tx_process.clone(), Some(
                Duration::from_secs(5))).await;
        for (_, r) in results {
            if let Some(r) = r.ok() {
                if let Some(obs_proof) = r.query_observation_proof_response {
                    observation_proofs.extend(obs_proof.observation_proof);
                }
            }
        }

        observation_proofs.extend(self.relay.ds.observation.select_observation_edge(&hash).await?);

        let mut submit_response = SubmitTransactionResponse::default();
        let mut query_transaction_response = QueryTransactionResponse::default();
        query_transaction_response.observation_proofs = observation_proofs.iter().map(|o| o.clone()).collect_vec();
        submit_response.query_transaction_response = Some(query_transaction_response);
        Ok(submit_response)
    }

    async fn insert_transaction(
        &self,
        hash: &Hash,
        transaction: &Transaction,
    ) -> Result<(), ErrorInfo> {
        // self.observation_buffer.push();
        // Commit transaction internally to database.

        // TODO: Handle these errors properly.
        // Add loggers, do as a single commit with a rollback.
        // Preserve all old data / inputs while committing new transaction, or do retries?
        // or fail the entire node?
        self.relay
                .ds
                .transaction_store
                .insert_transaction(
                    &transaction.clone(), util::current_time_millis_i64(), true, None
                ).await?;

        for (output_index, input) in transaction.inputs.iter().enumerate() {
            // This should maybe just put the whole node into a corrupted state? Or a retry?
            DataStore::map_err(
                self.relay.ds.delete_utxo(
                    &input
                        .transaction_hash
                        .as_ref()
                        .clone()
                        .expect("hash")
                        .bytes
                        .safe_bytes()
                        .expect("yes"),
                    output_index as u32,
                ),
            )?;
        }

        return Ok(());
    }

    fn clean_utxo(
        &self,
        request_processor: &RequestProcessor,
        utxo_ids: &Vec<FixedUtxoId>,
        i: Option<usize>,
    ) {
        // Cleanup all existing locks.
        // TODO: How to slice utxo_ids to 0..i ?
        let i = i.unwrap_or(utxo_ids.len());
        for utxo_id_cleanup in &utxo_ids[0..i] {
            let cleanup = self.relay.utxo_channels.get_mut(utxo_id_cleanup);
            match cleanup {
                None => {}
                Some(mut pool) => {
                    let idx = pool
                        .active_requests
                        .iter()
                        .enumerate()
                        .find(|(_, req)| {
                            req.request_processer.request_id == request_processor.request_id
                        })
                        .map(|x| x.0);
                    match idx {
                        None => {}
                        Some(idx_actual) => {
                            pool.active_requests.remove(idx_actual);
                        }
                    }
                }
            }
        }
    }

    fn create_receiver_or_err(
        &self,
        transaction_hash: &Hash,
        request_uuid: String,
    ) -> Result<RequestProcessor, ErrorInfo> {
        let entry = self
            .relay
            .transaction_channels
            .entry(transaction_hash.clone());
        let res = match entry {
            Entry::Occupied(_) => Err(error_message(Error::TransactionAlreadyProcessing, "Duplicate TX hash found in processing queue")),
            Entry::Vacant(entry) => {
                let req = RequestProcessor::new(&transaction_hash, request_uuid);
                entry.insert(req.clone());
                Ok(req)
            }
        };
        res
    }
}

// TODO: really just make this a full proper wallet keeping track of all this info.
// all these are broke ?
//
// #[test]
// fn test_processing() {
//     util::init_logger();
//     let ttc = TransactionTestContext::default();
//     let ds = ttc.relay.ds.clone();
//     info!("Data store path in outer context {}", ds.connection_path);
//     let utxos = ds.query_utxo_all_debug().unwrap();
//     info!(
//         "Data store outside genesis query {}",
//         serde_json::to_string(&utxos.clone()).unwrap()
//     );
//     for u in utxos {
//         info!(
//             "{:?}, {}",
//             rounded_balance(u.amount()),
//             hex::encode(u.address)
//         )
//     }
//     info!("{:?}", ds.query_all_balance().unwrap());
//     info!(
//         "ttc.t transaction ser: {}",
//         serde_json::to_string(&ttc.t.clone()).unwrap()
//     );
//     info!("{:?}", ttc.t.clone().output_amounts());
//     info!(
//         "genesis hash hex {:?}",
//         create_genesis_transaction().hash_hex()
//     );
//
//     let inp = ttc.t.iter_utxo_inputs().clone().get(0).unwrap().clone().0;
//     info!("Inputs of transaction: {}", hex::encode(inp.clone()));
//
//     info!("Query utxo of above: {:?}", ds.query_utxo(&inp, 0));
//
//     ttc.t.clone().validate_currency_utxo(&ds.clone()).unwrap();
//
//     let submitter = build_runtime(1, "test_tx_gen");
//
//     let tpc = TransactionProcessContext {
//         relay: ttc.relay.clone(),
//         tx_process: build_runtime(1, "test_tx_proc"),
//     };
//
//     let res = submitter.block_on(tpc.process(&ttc.t.clone()));
//     if res.is_err() {
//         info!("Error encountered {:?}", res.err());
//     }
//     assert!(res.is_ok());
//     for (hash, i) in ttc.t.iter_utxo_outputs() {
//         let res = tpc.relay.ds.query_utxo(&hash, i).unwrap();
//         assert!(res.is_some());
//     }
//     info!("{:?}", ds.query_all_balance().unwrap());
//     let utxos2 = ds.query_utxo_all_debug().unwrap();
//     info!(
//         "Data store outside genesis query {}",
//         serde_json::to_string(&utxos2.clone()).unwrap()
//     );
//     for u in utxos2.clone() {
//         info!(
//             "{:?}, {}",
//             rounded_balance(u.amount()),
//             hex::encode(u.address)
//         )
//     }
//     assert_eq!(utxos2.len(), 2);
// }
//
// #[test]
// fn test_duplicate_tx_hash() {
//     util::init_logger();
//     let ttc = TransactionTestContext::default();
//     let ds = ttc.relay.ds.clone();
//
//     let submitter = build_runtime(10, "test_tx_gen");
//
//     let tpc = TransactionProcessContext {
//         relay: ttc.relay.clone(),
//         tx_process: build_runtime(10, "test_tx_proc"),
//     };
//
//     let res = submitter.spawn({
//         let context = tpc.clone();
//         let transaction = ttc.t.clone();
//         async move { context.process(&transaction).await }
//     });
//     let res2 = submitter.spawn({
//         let context = tpc.clone();
//         let transaction = ttc.t.clone();
//         async move { context.process(&transaction).await }
//     });
//     let r1 = submitter.block_on(res).unwrap();
//     let r2 = submitter.block_on(res2).unwrap();
//
//     info!("{:?}", r1);
//     info!("{:?}", r2);
//
//     let results = vec![r1, r2];
//     assert!(results.iter().find(|x| x.is_err()).is_some());
//     assert!(results.iter().find(|x| x.is_ok()).is_some());
// }
//
// // Re-enable later
// #[ignore]
// #[test]
// fn test_double_spend() {
//     util::init_logger().ok();
//     let submitter = build_runtime(10, "test_tx_gen");
//     let mut ttc = submitter.block_on(TransactionTestContext::default());
//     let ds = ttc.relay.ds.clone();
//     info!("Addr balance: {:?}", ds.query_balance(&ttc.tc.addr.clone()));
//
//     info!("{:?}", ds.query_all_balance());
//
//     let tpc = TransactionProcessContext {
//         relay: ttc.relay.clone(),
//         tx_process: build_runtime(10, "test_tx_proc"),
//     };
//
//     let (dbl1, dbl2) = ttc.tx_gen.generate_double_spend_tx();
//
//     let res = submitter.spawn({
//         let context = tpc.clone();
//         let transaction = dbl1.clone().transaction;
//         async move { context.process(&transaction).await }
//     });
//     let res2 = submitter.spawn({
//         let context = tpc.clone();
//         let transaction = dbl2.clone().transaction;
//         async move { context.process(&transaction).await }
//     });
//     let r1 = submitter.block_on(res).unwrap();
//     let r2 = submitter.block_on(res2).unwrap();
//
//     info!("{:?}", r1);
//     info!("{:?}", r2);
//     info!("{:?}", ds.query_all_balance());
//
//     let results = vec![r1, r2];
//     assert!(results.iter().find(|x| x.is_err()).is_some());
//     assert!(results.iter().find(|x| x.is_ok()).is_some());
// }

// https://docs.rs/crossbeam-channel/0.5.1/crossbeam_channel/index.html
// select timeout after duration
// try iter
// try non blocking async
