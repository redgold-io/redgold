use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

// use crossbeam_channel::{unbounded, Receiver, Sender};
use dashmap::mapref::entry::Entry;
use flume::{Sender, TryRecvError};
use futures::{TryFutureExt, TryStreamExt};
use itertools::Itertools;
use log::{debug, error, info};
use metrics::{counter, histogram};
use tokio::runtime::Runtime;
use tokio::select;
use tokio::task::{JoinError, JoinHandle};
use uuid::Uuid;
use redgold_schema::{RgResult, SafeOption, struct_metadata_new, structs, task_local, task_local_map};
use redgold_schema::structs::{ContentionKey, ContractStateMarker, ExecutionInput, ExecutorBackend, GossipTransactionRequest, Hash, PublicResponse, QueryObservationProofRequest, Request, Response, UtxoId, ValidationType};

use crate::core::internal_message::{Channel, new_bounded_channel, PeerMessage, RecvAsyncErrorInfo, SendErrorInfo, TransactionMessage};
use crate::core::relay::Relay;
use crate::core::transaction::TransactionTestContext;
use redgold_data::data_store::DataStore;
use crate::schema::structs::{ErrorCode, ResponseMetadata};
use crate::schema::structs::{HashType, ObservationMetadata, State, Transaction};
use crate::schema::structs::{QueryTransactionResponse, SubmitTransactionResponse};
use crate::util::runtimes::build_runtime;
// TODO config
use crate::schema::structs::ErrorInfo;
use crate::schema::structs::ObservationProof;
use crate::schema::{empty_public_response, error_info, error_message};
use crate::util;
use futures::{stream::FuturesUnordered, StreamExt};
use redgold_executor::extism_wrapper;
use redgold_keys::proof_support::ProofSupport;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::tx_proof_validate::TransactionProofValidator;
use redgold_schema::output::tx_output_data;
use crate::core::resolver::resolve_transaction;
use crate::core::transact::utxo_conflict_resolver::check_utxo_conflicts;
use crate::util::current_time_millis_i64;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::easy_json::json_or;
use redgold_schema::helpers::with_metadata_hashable::{WithMetadataHashable, WithMetadataHashableFields};
use redgold_schema::observability::errors::EnhanceErrorInfo;
use crate::core::transact::contention_conflicts::{ContentionMessageInner, ContentionResult};
use crate::core::transact::tx_validate::TransactionValidator;
use crate::core::transact::tx_writer::{TransactionWithSender, TxWriterMessage};
use redgold_schema::observability::errors::Loggable;
use redgold_schema::proto_serde::{ProtoHashable, ProtoSerde};

#[derive(Clone)]
pub struct Conflict {
    //  Not really necessary but put other info here.
    transaction_hash: Hash,
    abort: bool,
    processing_start_time: i64,
    request_processer: RequestProcessor,
}

#[derive(Clone)]
pub enum ProcessTransactionMessage {
    ProofReceived(ObservationProof),
    // TODO: Do we need a type here for representing an immediate pending signature? probably not
}

#[derive(Clone)]
pub struct RequestProcessor {
    sender: flume::Sender<Conflict>,
    receiver: flume::Receiver<Conflict>,
    request_id: String,
    pub transaction_hash: Hash,
    pub transaction: Transaction,
    pub internal_channel: Channel<ProcessTransactionMessage>
}

#[derive(Clone)]
pub struct UTXOContentionPool {
    active_requests: Vec<Conflict>,
}

impl RequestProcessor {
    fn new(transaction_hash: &Hash, request_id: String, transaction: Transaction) -> RequestProcessor {
        let (s, r) = flume::unbounded::<Conflict>();
        return RequestProcessor {
            sender: s,
            receiver: r,
            request_id,
            transaction_hash: transaction_hash.clone(),
            transaction,
            internal_channel: new_bounded_channel(200),
        };
    }
}

#[derive(Clone)]
pub struct TransactionProcessContext {
    relay: Relay,
    request_processor: Option<RequestProcessor>,
    transaction_hash: Option<Hash>,
    utxo_ids: Option<Vec<UtxoId>>
}


// TODO: Refactor
async fn resolve_conflict(relay: Relay, conflicts: Vec<Conflict>) -> Result<Hash, ErrorInfo> {
    //relay.ds.select_peer_trust()
    // TODO: Add hasher interface to relay / node_config
    // let leaf_hash = util::dhash_vec(&transaction_hash).to_vec();

    // TODO: add fields pct_acceptance network 0.8 for instance fraction of peers / fraction of trust accepted.
    // TODO: Trust within partition id. partitions should be flexible in size relative to node size.
    // or rather node size determines minimum -- partition?

    // TODO: replace this func
    // relay.ds.peer_store.node_peer_id_trust()
    let an = relay.ds.peer_store.active_nodes(None).await?;
    // let peer_publics = relay.ds.select_broadcast_peers().unwrap();
    let mut map = HashMap::<Vec<u8>, f64>::new();
    for peer in &an {
        let t = relay.get_security_rating_trust_of_node(peer).await?.unwrap_or(0.0);
        map.insert(peer.vec(), t);
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
                    .map(|p| p.public_key_proto_bytes().as_ref().unwrap().clone())
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
    pub fn new(relay: Relay
               // , tx_process_listener: Arc<Runtime>
               // , tx_process: Arc<Runtime>
    ) -> JoinHandle<Result<(), ErrorInfo>> {
        let context = Self {
            relay,
            // tx_process: tx_process.clone(),
            request_processor: None,
            transaction_hash: None,
            utxo_ids: None,
        };

        return tokio::spawn(async move { context.run().await });
    }

    /// Loop to check messages and process them
    // TODO: Abstract this out
    async fn run(&self) -> Result<(), ErrorInfo> {
        counter!("redgold.node.async_started").increment(1);
        // let mut fut = FutLoopPoll::new();
        // TODO: Change to queue
        let receiver = self.relay.transaction_process.receiver.clone();
        // TODO this accomplishes queue, we can also move the spawn function into FutLoopPoll so that
        // the only function we have here is to call one function
        // do that later
        // receiver.stream().try_for_each_concurrent()
        // We can also use a Context type parameter over it to keep track of some internal context
        // per request i.e. RequestContext.
        receiver.into_stream().map(|x| {
            // info!("Transaction receiver map stream");
            Ok(x)
        })
            .try_for_each_concurrent(self.relay.node_config.tx_config.concurrency, |transaction| {
            // info!("Transaction receiver try for each stream");
            let mut x = self.clone();
            async move {
                x.scoped_process_and_respond(transaction).await
            }
        }).await
    }

    async fn scoped_process_and_respond(&mut self, mut transaction_message: TransactionMessage) -> Result<(), ErrorInfo> {

        counter!("redgold_process_transaction_called").increment(1);
        transaction_message.transaction.with_hashes();
        let request_uuid = Uuid::new_v4().to_string();
        let hex = transaction_message.transaction.calculate_hash().hex();
        let time = transaction_message.transaction.time().map(|x| x.clone()).unwrap_or(0);
        let current_time = util::current_time_millis_i64();
        let input_address = transaction_message.transaction.first_input_address().clone()
            .and_then(|a| a.render_string().ok()).unwrap_or("".to_string());
        let output_address = transaction_message.transaction.first_output_address_non_input_or_fee()
            .and_then(|a| a.render_string().ok()).unwrap_or("".to_string());
        let node_id = self.relay.node_config.short_id()?;
        let mut hm = HashMap::new();
        hm.insert("request_uuid".to_string(), request_uuid.clone());
        hm.insert("transaction_hash".to_string(), hex.clone());
        hm.insert("transaction_time".to_string(), time.to_string());
        hm.insert("current_time".to_string(), current_time.to_string());
        hm.insert("input_address".to_string(), input_address.clone());
        hm.insert("output_address".to_string(), output_address.clone());
        hm.insert("node_id".to_string(), node_id.clone());

        let res = task_local_map(hm, async move {
            self.transaction(
                transaction_message, request_uuid, hex,
                time, current_time, input_address, output_address,
                node_id
            ).await
        }).await;
        res
    }

    /*
    let span = Span::current();
    span.record("extra_field", &"extra_value");
     */

    #[allow(unused_variables)]
    #[tracing::instrument(skip(self, transaction_message, transaction_time, current_time, input_address, output_address))]
    async fn transaction(
        &mut self,
        transaction_message: TransactionMessage,
        request_uuid: String,
        transaction_hash: String,
        transaction_time: i64,
        current_time: i64,
        input_address: String,
        output_address: String,
        node_id: String
    ) -> Result<(), ErrorInfo> {

        self.transaction_hash = Some(transaction_message.transaction.hash_or());

        if self.check_peer_message(&transaction_message.transaction).await? {
            return self.process_peer_transaction(&transaction_message.transaction).await;
        }

        let result_or_error = {
            let result = match self.immediate_validation(&transaction_message.transaction).await {
                Ok(_) => {
                    let res = self.process(transaction_message.transaction.clone(), current_time, request_uuid).await
                        .with_detail("transaction", transaction_message.transaction.json_or());
                    res
                }
                Err(e) => {
                    Err(e)
                }
            };
            self.cleanup(None)?;
            result
        };

        // Use this as whether or not the request was successful
        let mut metadata = ResponseMetadata::default();
        // Change these to raw Response instead of public response
        let mut pr = structs::Response::default();
        match result_or_error.log_error() {
            Ok(o) => {
                metadata.success = true;
                counter!("redgold_process_transaction_success", &self.relay.node_config.gauge_id()).increment(1);
                pr.submit_transaction_response = Some(o);
            }
            Err(ee) => {
                metadata.success = false;
                counter!("redgold_process_transaction_failure", &self.relay.node_config.gauge_id()).increment(1);
                metadata.error_info = Some(ee);
            }
        }
        pr.response_metadata = Some(metadata);

        match transaction_message.response_channel {
            None => {
                // counter!("redgold.transaction.missing_response_channel").increment(1);
                // let details = ErrorInfo::error_info("Missing response channel for transaction");
                // error!("Missing response channel for transaction {:?}", json_or(&details));
            }
            Some(r) => if let Some(e) = r.send_rg_err(pr).err() {
                error!("Error sending transaction response to channel {}", json_or(&e));
            }
        };
        Ok(())
    }

    // TODO: okay so in this loop below thre's a time which should also check if all
    // available peers have heard about a double spend etc. and potentially terminate quicker?
    // maybe? or not hell not really necessary.

    fn cleanup(&mut self, ii: Option<usize>) -> Result<(), ErrorInfo> {
        self.relay.transaction_channels.remove(self.transaction_hash.safe_get()?);
        if let Some(request_processor) = &self.request_processor {
            if let Some(utxo_ids) = &self.utxo_ids {
                self.clean_utxo(&request_processor, utxo_ids, ii);
            }
        }
        Ok(())
    }

    async fn observe(&self, validation_type: ValidationType, state: State) -> Result<ObservationProof, ErrorInfo> {
        let hash: Hash = self.transaction_hash.safe_get()?.clone();
        // TODO: It might be nice to grab the proof of a signature here?
        self.relay.observe_tx(&hash, state, validation_type, structs::ValidationLiveness::Live).await
    }

    async fn immediate_validation(&mut self, transaction: &Transaction) -> Result<(), ErrorInfo> {

        // Check if we already have a rejection reason for this transaction and abort if so
        // returning the previous rejection reason.
        let ds = self.relay.ds.clone();
        if let Some((_, Some(pre_rejection))) = ds.transaction_store.query_maybe_transaction(&transaction.hash_or()).await? {
            return Err(pre_rejection);
        }

        // Validate obvious schema related errors / local errors requiring no other context information
        transaction.validate(Some(&self.relay.node_config.seed_peer_addresses()), Some(&self.relay.node_config.network))?;
        Ok(())

    }

    // TODO: Add a debug info thing here? to include data about debug calls? Thread local info? something ?
    async fn process(&mut self, mut transaction: Transaction, processing_time_start: i64, request_uuid: String) -> Result<SubmitTransactionResponse, ErrorInfo> {
        counter!("redgold.transaction.received").increment(1);
        let hash = transaction.hash_or();
        self.transaction_hash = Some(hash.clone());

        histogram!("redgold.transaction.size_bytes").record(transaction.proto_serialize().len() as f64);
        histogram!("redgold.transaction.total_output_amount").record(transaction.total_output_amount_float());
        histogram!("redgold.transaction.floating_inputs").record(transaction.floating_inputs().count() as f64);
        histogram!("redgold.transaction.num_inputs").record(transaction.inputs.len() as f64);
        histogram!("redgold.transaction.num_outputs").record(transaction.outputs.len() as f64);

        // Establish channels for other transaction threads to communicate conflicts with this one.
        let request_processor = self.create_receiver_or_err(&hash, request_uuid, &transaction)?;

        // Attempt to resolve all the transaction inputs and outputs for context-aware validation
        // This is the place where balances checks and signature verifications are performed.
        let resolver_data = resolve_transaction(&transaction,
                                                self.relay.clone(),
                                                // self.tx_process.clone()
        ).await?;
        resolver_data.validate_input_output_amounts_match()?;
        transaction = resolver_data.with_enriched_inputs()?;

        let fixed_utxo_ids = transaction.fixed_utxo_ids_of_inputs()?;
        self.utxo_ids = Some(fixed_utxo_ids.clone());

        // Enable UTXO contention buckets
        // let mut contention_responses = vec![];
        // for u in &fixed_utxo_ids {
        //     let mut ck = ContentionKey::default();
        //     ck.utxo_id = Some(u.clone());
        //     let msg = ContentionMessageInner::RegisterPotentialContention {
        //         transaction_hash: hash.clone()
        //     };
        //     contention_responses.push(self.relay.contention_message(&ck, msg).await?);
        // }

        // TODO: Check for conflicts via peer query -- currently unimplemented
        check_utxo_conflicts(self.relay.clone(), &fixed_utxo_ids, &hash).await?;

        let mut conflict_detected = false;
        let self_conflict = Conflict {
            transaction_hash: hash.clone(),
            abort: false,
            processing_start_time: processing_time_start.clone(),
            request_processer: request_processor.clone(),
        };
        let mut conflicts: Vec<Conflict> = vec![];

        // Change to UTXO stream processor sink message?
        // TODO: Change this so the UTXO pool is responsible for this, remove the request processor from here
        // And create a spawned thread for each UTXO pool to handle this that returns any conflicts back here.
        for utxo_id in fixed_utxo_ids {
            // Acquire locks to determine if another transaction is re-using some output
            let entry = self.relay.utxo_channels.entry(utxo_id.clone());
            match entry {
                Entry::Occupied(mut entry) => {
                    conflict_detected = true;
                    // issue! another pending transaction is using this
                    // this indicates a double spend
                    //receiver.
                    for active_request in entry.get().active_requests.iter() {
                        // TODO: Handle unwrap issues
                        conflicts.push(active_request.clone());
                        info!(
                            "Conflict between current: {} and {} on utxo_id {} output: {}",
                            hash.hex(), active_request.transaction_hash.hex(),
                            utxo_id.clone().transaction_hash.expect("h").hex(),
                            utxo_id.output_index.clone()
                        );
                        // Need to capture this conflict LOCALLY here for use later.
                        active_request
                            .request_processer
                            .sender
                            .send_rg_err(self_conflict.clone())?
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
        }

        // TODO: Don't remember the purpose of this duplicate validation here, we should really
        // seed this with existing data to reduce request size and just verify it's still valid
        let resolver_data = resolve_transaction(&transaction,
                                                self.relay.clone(),
                                                // self.tx_process.clone()
        ).await?;
        resolver_data.validate_input_output_amounts_match()?;
        // Change this to 'revalidate' and only issue some queries again not all of them.

        let validation_type = if resolver_data.resolved_internally {
            ValidationType::Full
        } else {
            ValidationType::Partial
        };

        let mut observation_proofs = HashSet::new();
        let mut self_signed_pending = false;

        if !conflict_detected {
            // tracing::info!("Signing pending transaction");
            let prf = self.observe(validation_type, State::Pending).await?;
            self_signed_pending = true;
            observation_proofs.insert(prf);
            // TODO: Await the initial observations here, gossip about them as well?
        } else {
            tracing::info!("Conflict detected on current {}", hash.hex());
        }

        // tracing::info!("Pending transaction stage started");

        // We need to distinguish between a pending and a conflict detected here.
        // TODO: Broadcast with no concern about response, or do we check for conflicts here?
        // TODO: we really need here a mechanism to deal with incoming conflict notifications
        // A request type to handle that that'll alert this thread? OR should that just be the same
        // As transaction request? We might benefit from including additional information.
        // TODO: Replace all this with a relay method.
        self.relay.gossip(&transaction).await?;

        // tracing::info!("Gossiped transaction");

        let elapsed_time = || {
            current_time_millis_i64() - processing_time_start
        };

        // This thread should listen for any incoming messages that alert to conflicts
        // And or any messages that are related to proofs for this transaction.


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
                    return Err(error_message(ErrorCode::TransactionRejectedDoubleSpend, "Double spend detected in live context"));
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

        // TODO: Move to own function
        // if let Some(value) = Self::poll_internal_proof_messages(&request_processor, &mut observation_proofs) {
        //     value?;
        // }

        // Completion stage

        // tracing::info!("Conflict resolution stage started with {:?} conflicts", conflicts.len());

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
                Err(error_message(ErrorCode::TransactionRejectedDoubleSpend,
                                  format!("Lost conflict to other transaction winner: {}", winner.hex())))?;
            } else {
                for c in &conflicts {
                    c.request_processer
                        .sender
                        .send_rg_err(this_as_conflict.clone())?;
                }
            }
        }

        // Sanity check here, instead use a tombstone on Pending, or otherwise trigger a conflict resolution process.
        for u in transaction.utxo_inputs() {
            if !self.relay.ds.utxo.utxo_id_valid(&u).await? {
                Err(error_info("Aborting process transaction due to \
                UTXO id considered invalid immediately prior to acceptance after pending"))
                    .add(u.json_or())?
            }
            let child_opt = self.relay.ds.utxo.utxo_child(&u).await?;
            if let Some((child_hash, child_idx)) = child_opt {
                Err(error_info("Aborting process transaction due to \
                UTXO has child invocation immediately prior to acceptance after pending with child"))
                    .add(u.json_or())
                    .add(child_hash.hex())
                    .add(child_idx.to_string())?
            }
        }

        // tracing::info!("Finalizing transaction stage started");

        let prf = self.observe(validation_type, State::Accepted).await?;
        observation_proofs.insert(prf);
        // self.insert_transaction(&transaction).await?;
        // TODO: Use observation times and update later -- also add a txWriter type to update
        // tx time
        self.relay.write_transaction(&transaction, transaction.time()?.clone(), None, true).await?;
        counter!("redgold.transaction.accepted").increment(1);
        // tracing::info!("Accepted transaction");

        // tracing::info!("Finalize end on current {} with num conflicts {:?}", hash.hex(), conflicts.len());

        // Await until it has appeared in an observation and other nodes observations.


        let mut retries = 0;
        loop {
            retries += 1;
            tokio::time::sleep(Duration::from_secs(1)).await;
            let stored_proofs = self.relay.ds.observation.select_observation_edge(&hash).await?;
            // tracing::info!("Found {:?} stored proofs in ds", stored_proofs.len());
            observation_proofs.extend(stored_proofs);
            let pks = self.relay.node_config.seeds_now_pk();
            let done = pks.iter().all(|pk| {
                observation_proofs.iter()
                    .filter(|op| op.metadata.as_ref()
                        .map(|m| m.state() == State::Accepted).unwrap_or(false)
                    ).any(|o| {
                    o.proof.as_ref().and_then(|p| p.public_key.as_ref()).map(|p| p == pk).unwrap_or(false)
                })
            });
            if done || retries > 20 {
                break;
            }
        };

        // TODO: Query our internal datastore for all obs proofs, and extend based on that

        // TODO: periodic process to clean mempool in event of thread processing crash?
        let peers = self.relay.ds.peer_store.active_nodes(None).await?;
        let mut obs_proof_req = Request::default();
        obs_proof_req.query_observation_proof_request = Some(QueryObservationProofRequest {
            hash: Some(hash.clone())
        });

        if !peers.is_empty() {
            // tracing::info!("Collecting observation proofs from {} peers", peers.len());
            let results = Relay::broadcast(self.relay.clone(),
                                           peers, obs_proof_req,
                                           // self.tx_process.clone(),
                                           Some(
                                               Duration::from_secs(10))).await;
            for (pk, r) in results {
                match r {
                    Ok(r) => {
                        let num_proofs = r.clone().query_observation_proof_response.map(|o| o.observation_proof.len()).unwrap_or(0);
                        // tracing::info!("Received {:?} observation proofs from peer: {}", num_proofs,  pk.short_id());
                        if let Some(obs_proof) = r.query_observation_proof_response {
                            observation_proofs.extend(obs_proof.observation_proof);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error collecting observation proofs from peer: {} error: {}", pk.short_id(), e.json_or())
                    }
                }
            }
        } else {
            tracing::info!("No peers to collect observation proofs from");
        }

        observation_proofs.extend(self.relay.ds.observation.select_observation_edge(&hash).await?);

        let mut submit_response = SubmitTransactionResponse::default();
        let mut query_transaction_response = QueryTransactionResponse::default();
        query_transaction_response.observation_proofs = observation_proofs.iter().map(|o| o.clone()).collect_vec();
        submit_response.query_transaction_response = Some(query_transaction_response);
        submit_response.transaction = Some(transaction.clone());
        submit_response.transaction_hash = Some(hash.clone());

        // Here now we need to send this transaction to a contract state manager if it's appropriate

        // For now this is the 'deploy' operation -- but it's not correct / validated yet.
        for o in &transaction.outputs {
            if let Some(c) = o.contract
                .as_ref().and_then(|c| c.code_execution_contract.as_ref()) {
                if let Some(b) = c.executor {
                    if b == (ExecutorBackend::Extism as i32) {
                        if let Some(code) = &c.code {
                            let mut input = ExecutionInput::default();
                            input.tx = Some(transaction.clone());
                            debug!("Invoking deploy contract call");
                            let er = extism_wrapper::invoke_wasm(
                                &*code.value,
                                "extism_entrypoint",
                                input
                            ).await?;
                            let mut csm = ContractStateMarker::default();
                            csm.state = er.data.as_ref()
                                .and_then(|d| d.state.clone());
                            csm.address = o.address.clone();
                            csm.time = transaction.time()?.clone();
                            csm.index_counter = 0;
                            csm.transaction_marker = Some(transaction.hash_or());
                            self.relay.ds.state.insert_state(csm).await?;
                            // TODO: ^ save the error above and return it to the user for processing this?
                            // we should know about this error well before we accept the transaction.
                        }
                    }
                }
            }
            if o.is_request() {
                let csm = self.relay.send_contract_ordering_message(&transaction, &o).await?;
                info!("Accepted CSM: {}", csm.json_or())
            }
        }

        // TODO: Task local metrics update here
        // let hm: HashMap<String, String> = HashMap::new();
        // hm
        let counts = submit_response.count_unique_by_state()?;

        tracing::info!("Finished processing transaction \
        num_observation_proofs {} self_signed_pending {:?} num_pending: {:?} num_accepted: {:?}",
            observation_proofs.len(), self_signed_pending, counts.get(&(State::Pending as i32)).unwrap_or(&0),
            counts.get(&(State::Accepted as i32)).unwrap_or(&0),
        );
        // TODO: Rename Finalized to Accepted

        Ok(submit_response)
    }

    // TODO: WArning this is actually wrong and will break everything, fix the done condition
    fn poll_internal_proof_messages(request_processor: &RequestProcessor, observation_proofs: &mut HashSet<ObservationProof>) -> Option<Result<SubmitTransactionResponse, ErrorInfo>> {
        while {
            let err = request_processor.internal_channel.receiver.try_recv();
            let mut done = true;
            match err {
                Ok(o) => {
                    match o {
                        ProcessTransactionMessage::ProofReceived(o) => {
                            observation_proofs.insert(o);
                        }
                    }
                }
                Err(e) => {
                    match e {
                        TryRecvError::Empty => {
                            done = true;
                        }
                        TryRecvError::Disconnected => {
                            return Some(Err(error_info("request processor channel closed unexpectedly")));
                        }
                    }
                }
            }
            done
        } {}
        None
    }

    fn clean_utxo(
        &self,
        request_processor: &RequestProcessor,
        utxo_ids: &Vec<UtxoId>,
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
        transaction: &Transaction
    ) -> Result<RequestProcessor, ErrorInfo> {
        let entry = self
            .relay
            .transaction_channels
            .entry(transaction_hash.clone());
        let res = match entry {
            Entry::Occupied(_) => Err(error_message(ErrorCode::TransactionAlreadyProcessing, "Duplicate TX hash found in processing queue")),
            Entry::Vacant(entry) => {
                let req = RequestProcessor::new(&transaction_hash, request_uuid, transaction.clone());
                entry.insert(req.clone());
                Ok(req)
            }
        };
        res
    }
    async fn check_peer_message(&self, p0: &Transaction) -> Result<bool, ErrorInfo> {
        let is_peer_tx = p0.node_metadata().is_ok() || p0.peer_data().is_ok();
        if is_peer_tx {
            // Ignore for now, causes a giant loop think we need to avoid gossiping about self?
            // self.relay.add_peer_flow(p0).await.log_error().ok();
        }
        Ok(is_peer_tx)
    }
    async fn process_peer_transaction(&self, tx: &Transaction) -> RgResult<()> {
        if let Some(_nmd) = tx.node_metadata().ok() {
            // TODO: Validate this update
            // self.relay.ds.peer_store.insert_node(tx).await?;
        } else if let Some(_pd) = tx.peer_data().ok() {
            // self.relay.ds.peer_store.insert_peer(tx).await?;
        }
        Ok(())
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
