use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crossbeam_channel::{unbounded, Receiver, Sender};
use dashmap::mapref::entry::Entry;
use futures::TryStreamExt;
use itertools::Itertools;
use log::{debug, error, info};
use metrics::increment_counter;
use tokio::runtime::Runtime;
use tokio::select;
use tokio::task::{JoinError, JoinHandle};
use uuid::Uuid;
use redgold_schema::struct_metadata_new;
use redgold_schema::structs::{GossipTransactionRequest, Hash, Request};

use crate::core::internal_message::{FutLoopPoll, PeerMessage, RecvAsyncErrorInfo, TransactionMessage};
use crate::core::relay::Relay;
use crate::core::transaction::{TransactionTestContext, validate_utxo};
use crate::data::data_store::DataStore;
use crate::schema::structs::{Error, ResponseMetadata};
use crate::schema::structs::{HashType, ObservationMetadata, State, Transaction};
use crate::schema::structs::{QueryTransactionResponse, SubmitTransactionResponse};
use crate::schema::{SafeBytesAccess, WithMetadataHashable};
use crate::util::runtimes::build_runtime;
use crate::schema::{error_from_code};
// TODO config
use crate::schema::structs::ErrorInfo;
use crate::schema::structs::ObservationProof;
use crate::schema::{empty_public_response, error_info, error_message};
use crate::util;
use futures::{stream::FuturesUnordered, StreamExt};
use crate::core::resolver::resolve_transaction;

#[derive(Clone)]
pub struct Conflict {
    //  Not really necessary but put other info here.
    transaction_hash: Vec<u8>,
    abort: bool,
    processing_start_time: SystemTime,
    request_processer: RequestProcessor,
}

#[derive(Clone)]
pub struct RequestProcessor {
    sender: flume::Sender<Conflict>,
    receiver: flume::Receiver<Conflict>,
    request_id: Uuid,
    pub transaction_hash: Vec<u8>,
}

#[derive(Clone)]
pub struct UTXOContentionPool {
    active_requests: Vec<Conflict>,
}

impl RequestProcessor {
    fn new(transaction_hash: Vec<u8>) -> RequestProcessor {
        let (s, r) = flume::unbounded::<Conflict>();
        return RequestProcessor {
            sender: s,
            receiver: r,
            request_id: Uuid::new_v4(),
            transaction_hash,
        };
    }
}

#[derive(Clone)]
pub struct TransactionProcessContext {
    relay: Relay,
    tx_process: Arc<Runtime>,
}

async fn query_self_accepted_transaction_status(
    relay: Relay,
    transaction_hash: Vec<u8>,
) -> Result<QueryTransactionResponse, ErrorInfo> {
    //relay.ds.select_peer_trust()
    // TODO: Add hasher interface to relay / node_config
    // let leaf_hash = util::dhash_vec(&transaction_hash).to_vec();

    // TODO: add fields pct_acceptance network 0.8 for instance fraction of peers / fraction of trust accepted.
    // TODO: Trust within partition id. partitions should be flexible in size relative to node size.
    // or rather node size determines minimum -- partition?

    let peer_publics = DataStore::map_err(relay.ds.select_broadcast_peers())?;

    let mut pk = peer_publics
        .iter()
        .map(|x| x.public_key.clone())
        .collect::<HashSet<Vec<u8>>>();

    pk.insert(relay.node_config.wallet().transport_key().public_key_vec());

    if pk.is_empty() {
        error!("Peer keys empty, unable to validate transaction!")
    }

    let mut interval =
        tokio::time::interval(relay.node_config.check_observations_done_poll_interval);

    let mut res: Vec<ObservationProof> = vec![];
    // what we should do is instead, in the process thing, cause a delay if we don't get any
    // new information.

    for _ in 0..relay.node_config.check_observations_done_poll_attempts {
        interval.tick().await;
        res = relay
            .ds
            .query_observation_edge(transaction_hash.clone())
            .unwrap();
        info!(
            "Queried {:?} merkle proofs for transaction_hash {}",
            res.len(),
            hex::encode(transaction_hash.clone())
        );
        // TODO: Change to trust threshold for acceptance.
        if res.len() >= 1 {
            break;
        }
    }
    Ok(QueryTransactionResponse {
        observation_proofs: res,
        block_hash: None,
    })
}

async fn resolve_conflict(relay: Relay, conflicts: Vec<Conflict>) -> Result<Vec<u8>, ErrorInfo> {
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

    let mut trust_conflicts = conflicts
        .iter()
        .map(|c| {
            let res = relay
                .ds
                .query_observation_edge(c.transaction_hash.to_vec().clone())
                .unwrap();
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
            (
                (-1 as f64) * sum_trust,
                c.processing_start_time,
                c.transaction_hash.to_vec(),
            )
        })
        .collect::<Vec<(f64, SystemTime, Vec<u8>)>>();
    trust_conflicts.sort_by(|x, y| x.partial_cmp(y).unwrap());
    Ok(trust_conflicts.get(0).unwrap().2.clone())
}

impl TransactionProcessContext {
    pub fn new(relay: Relay, tx_process_listener: Arc<Runtime>, tx_process: Arc<Runtime>) -> JoinHandle<Result<(), ErrorInfo>> {
        let context = Self {
            relay,
            tx_process: tx_process.clone(),
        };

        return tx_process_listener.spawn(async move { context.run().await });
    }

    async fn run(&self) -> Result<(), ErrorInfo> {
        increment_counter!("redgold.node.async_started");
        info!("TransactionProcessContextRanHere");

        let mut fut = FutLoopPoll::new();

        let mut receiver = self.relay.transaction.receiver.clone();
        fut.run_fut(&|| receiver.recv_async_err(), |transaction_res: Result<TransactionMessage, ErrorInfo>| {
            increment_counter!("redgold.transaction.received");
            let x = self.clone();
            let jh = self.tx_process.clone()
            .spawn(async move {
                match transaction_res {
                    Ok(transaction) => {
                        x.process_and_respond(transaction).await
                    }
                    Err(e) => Err(e)
                }
            });
            jh
        } ).await

        //
        // fut.run(self.relay.transaction.receiver.clone(), |transaction: TransactionMessage| {
        //     increment_counter!("redgold.transaction.received");
        //     let x = self.clone();
        //     let jh = self.tx_process.clone()
        //     .spawn(async move { x.process_and_respond(transaction).await });
        //     jh
        // } ).await
        //
        //
        //
        // let mut futures = FuturesUnordered::new();
        //
        // loop {
        //     let loop_sel_res = select! {
        //         transaction_res = self.relay.transaction.receiver.recv_async_err() => {
        //             let transaction: TransactionMessage = transaction_res?;
        //             info!("Futures length: {:?}, Received transaction in process context {}",
        //                 futures.len(),
        //                 transaction.clone().transaction.hash_hex_or_missing()
        //             );
        //             increment_counter!("redgold.transaction.received");
        //
        //             let x = self.clone();
        //             futures.push(self.tx_process
        //                 .clone()
        //                 .spawn(async move { x.process_and_respond(transaction).await }));
        //             Ok(())
        //         }
        //         res = futures.next() => {
        //             let r: Option<Result<Result<(), ErrorInfo>, JoinError>> = res;
        //             match r {
        //                 None => {
        //                     Ok(())
        //                 }
        //                 Some(resres) => {
        //                     resres.map_err(|je| ErrorInfo::error_info(
        //                         format!("Panic in transaction thread {}", je.to_string())
        //                     ))??;
        //                     Ok(())
        //                 }
        //             }
        //         }
        //     };
        //     loop_sel_res?;
        // }
        //
    }

    async fn post_process_query(
        &self,
        transaction_message: TransactionMessage,
    ) -> Result<SubmitTransactionResponse, ErrorInfo> {
        let result1 = tokio::time::timeout(
            Duration::from_secs(30),
            self.process(&transaction_message.transaction),
        )
        .await
        .map_err(|e| error_info("timeout on transaction process"))?
        //.map_err(|e| error_message(e, "transaction process"))?;
        ?;
        let vec1 = transaction_message.transaction.hash_bytes()?;
        let status = tokio::time::timeout(
            Duration::from_secs(15),
            query_self_accepted_transaction_status(self.relay.clone(), vec1.clone()),
        )
        .await
        .map_err(|e| error_info("timeout on query transaction accepted status"))??;
        Ok(SubmitTransactionResponse {
            transaction_hash: transaction_message.transaction.hash().into(),
            query_transaction_response: Some(status),
        })
    }

    async fn process_and_respond(&self, transaction_message: TransactionMessage) -> Result<(), ErrorInfo> {
        let result2 = self.post_process_query(transaction_message.clone()).await;

        // Use this instead as whether or not the request was successful,
        // rather than info about the particular transaction.

        let metadata = ResponseMetadata {
            success: result2.is_ok(),
            error_info: match result2.clone() {
                Ok(_) => None,
                Err(e) => Some(e),
            },
        };
        let mut pr = empty_public_response();
        pr.response_metadata = Some(metadata);
        pr.submit_transaction_response = match result2 {
            Ok(e) => Some(e),
            Err(_) => None,
        };

        match transaction_message.response_channel {
            None => {
                // register error metric or log error?
                // abort process? kill node???
                // send node command kill signal.
                error!("Missing transaction message response channel!")
            }
            Some(r) => match r.send(pr) {
                Err(e) => {
                    error!(
                        "Error sending transaction response to API {:?}, {:?}, {:?}",
                        e,
                        e.to_string(),
                        e.0
                    )
                }
                Ok(_) => {}
            },
        };
        Ok(())
    }

    // TODO: okay so in this loop below thre's a time which should also check if all
    // available peers have heard about a double spend etc. and potentially terminate quicker?
    // maybe? or not hell not really necessary.

    // TODO: Add a debug info thing here? to include data about debug calls? Thread local info? something ?
    async fn process(&self, transaction: &Transaction) -> Result<QueryTransactionResponse, ErrorInfo> {
        let processing_time_start = SystemTime::now();
        let hash_vec = transaction.hash().safe_bytes()?;
        // TODO: Check if we've already rejected this transaction with an error with a LRU cache
        let request_processor = self.create_receiver_or_err(&hash_vec)?;
        // Validate obvious schema related errors
        transaction.prevalidate().map_err(error_from_code)?;

        let transaction1 = transaction.clone();
        let resolver_data = resolve_transaction(transaction1.clone(),
            self.relay.clone(), self.tx_process.clone()).await?;
        resolver_data.validate()?;

        let utxo_id_inputs = transaction.utxo_ids_of_inputs()?;

        // Resolve transaction inputs & trigger potential downloads for prior data.

        // Validate signatures and balances

        let utxo_ids = transaction.iter_utxo_inputs();
        // let utxo_ids = validate_utxo(transaction, &self.relay.ds)?;
        let utxo_ids2 = utxo_ids.clone();
        let mut i: usize = 0;

        let clean_up = |ii| {
            self.clean_utxo(&request_processor, &utxo_ids2, ii);
            self.relay.transaction_channels.remove(&hash_vec.clone());
            // self.mem_pool.transactions.remove(&hash);
        };

        let mut conflict_detected = false;
        let self_conflict = Conflict {
            transaction_hash: hash_vec.clone(),
            abort: false,
            processing_start_time: processing_time_start.clone(),
            request_processer: request_processor.clone(),
        };
        let mut conflicts: Vec<Conflict> = vec![];

        for utxo_id in utxo_ids {
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
                        info!(
                            "Conflict between current: {} and {}",
                            hex::encode(hash_vec.clone()),
                            hex::encode(active_request.transaction_hash.clone())
                        );
                        // Need to capture this conflict LOCALLY here for use later.
                        active_request
                            .request_processer
                            .sender
                            .send(self_conflict.clone())
                            .unwrap();
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

        // TODO: Don't remember the purpose of this duplicate validation here?
        let resolver_data = resolve_transaction(transaction1.clone(),
                                                self.relay.clone(), self.tx_process.clone()).await?;

        // match validate_utxo(transaction, &self.relay.ds) {
        match resolver_data.validate() {
            Ok(_) => {}
            Err(err) => {
                clean_up(i);
                return Err(err);
            }
        }

        if !conflict_detected {
            let mut hash: Hash = hash_vec.clone().into();
            hash.hash_type = HashType::Transaction as i32;
            self.relay
                .observation_metadata
                .sender
                .send(ObservationMetadata {
                    observed_hash: Some(hash),
                    hash: None,
                    state: Some(State::Pending as i32),
                    validation_confidence: None,
                    struct_metadata: redgold_schema::struct_metadata_new(),
                    observation_type: 0
                })
                .expect("Fail");
        } else {
            info!(
                "Conflict detected on current {}",
                hex::encode(hash_vec.clone())
            );
        }

        let mut request = self.relay.node_config.request();
        request.gossip_transaction_request = Some(GossipTransactionRequest {
            transaction: Some(transaction.clone()),
        });
        let mut message = PeerMessage::empty();
        message.request = request;
        self.relay
            .peer_message_tx
            .sender
            .send(message)
            .expect("send fail");
        // Now that we have channels associated with all the UTXOs, we can proceed
        // down the happy path (assuming we didn't hit Occupied above, which we'll handle after.)
        //
        // let watch = request_processor
        //     .receiver
        //     .recv_timeout(default_finalize_timeout);
        // let watch2 = watch.clone();
        // match watch {
        //     Ok(c) => {
        //         if c.abort {
        //             info!(
        //                 "Immediate abort detected on current {}",
        //                 hex::encode(hash.to_vec())
        //             );
        //             clean_up(i);
        //             return Err(Error::TransactionRejectedDoubleSpend);
        //         }
        //     }
        //     // This means we didn't detect any double spends
        //     Err(_) => {
        //         // TODO This step should also include something that sends abort codes if any other
        //         // locks exist.
        //         info!(
        //             "Finalize immediate due to no received conflicts detected on current {}",
        //             hex::encode(hash.to_vec())
        //         );
        //         self.finalize_transaction(&hash, transaction);
        //         clean_up(i);
        //         return Ok(());
        //     }
        // }

        // Unhappy path -- conflict detected.
        // Now we need to collect all conflict related data until we make a decision.

        let elapsed_time = || {
            SystemTime::now()
                .duration_since(processing_time_start)
                .unwrap()
        };

        // conflicts.push(watch2.unwrap());
        // let mut conflicts = vec![watch2.unwrap()];
        // Potentially receive abort code here to terminate thread early.
        // if another transaction was finalized.

        while elapsed_time() < self.relay.node_config.transaction_finalization_time {
            // TODO: timeout
            match request_processor.receiver.recv_timeout(Duration::new(1, 0)) {
                Ok(conflict) => {
                    if conflict.abort {
                        clean_up(i);
                        // translate error codes for db access / etc. into internal server error.
                        return Err(error_from_code(Error::TransactionRejectedDoubleSpend));
                    }
                    conflicts.push(conflict);
                }
                Err(_) => {
                    // Continue waiting for more results.
                }
            }
        }

        // let this_tx_trust = self.mem_pool.total_trust(&hash, &self.ds).unwrap_or(0f64);

        let this_as_conflict = Conflict {
            transaction_hash: hash_vec.clone(),
            abort: true,
            processing_start_time: processing_time_start.clone(),
            request_processer: request_processor.clone(),
        };
        let mut all_conflicts = conflicts.clone();
        all_conflicts.push(this_as_conflict.clone());

        let winner = resolve_conflict(self.relay.clone(), all_conflicts)
            .await
            .unwrap();

        if winner == hash_vec.clone() {
            for c in &conflicts {
                c.request_processer
                    .sender
                    .send(this_as_conflict.clone())
                    .expect("fail");
            }
            // TODO: Waht is the error here?
            self.finalize_transaction(&hash_vec, transaction).await?;
            // .expect("Bubble up later");
        } else {
            clean_up(i);
            // translate error codes for db access / etc. into internal server error.
            return Err(error_from_code(Error::TransactionRejectedDoubleSpend));
        }
        clean_up(i);

        info!(
            "Finalize end on current {} with num conflicts {:?}",
            hex::encode(hash_vec.clone()),
            conflicts.len()
        );

        // Await until it has appeared in an observation and other nodes observations.

        // TODO: periodic process to clean mempool in event of thread processing crash
        query_self_accepted_transaction_status(self.relay.clone(), hash_vec.clone()).await
    }

    async fn finalize_transaction(
        &self,
        hash: &Vec<u8>,
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

        self.relay
            .observation_metadata
            .sender
            .send(ObservationMetadata {
                hash: None, 
                observed_hash: Some(hash.to_vec().into()),
                state: Some(State::Finalized as i32),
                validation_confidence: None,
                struct_metadata: struct_metadata_new(),
                observation_type: 0
            })
            .expect("sent");
        info!(
            "Accepted transaction: {}",hex::encode(hash.clone())
        );
        increment_counter!("redgold.transaction.accepted");
        return Ok(());
    }

    fn clean_utxo(
        &self,
        request_processor: &RequestProcessor,
        utxo_ids: &Vec<(Vec<u8>, i64)>,
        i: usize,
    ) {
        // Cleanup all existing locks.
        // TODO: How to slice utxo_ids to 0..i ?
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
        transaction_hash: &Vec<u8>,
    ) -> Result<RequestProcessor, ErrorInfo> {
        let entry = self
            .relay
            .transaction_channels
            .entry(transaction_hash.clone());
        let res = match entry {
            Entry::Occupied(_) => Err(error_from_code(Error::TransactionAlreadyProcessing)),
            Entry::Vacant(entry) => {
                let req = RequestProcessor::new(transaction_hash.clone());
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

// Re-enable later
#[ignore]
#[test]
fn test_double_spend() {
    util::init_logger().ok();
    let submitter = build_runtime(10, "test_tx_gen");
    let mut ttc = submitter.block_on(TransactionTestContext::default());
    let ds = ttc.relay.ds.clone();
    info!("Addr balance: {:?}", ds.query_balance(&ttc.tc.addr.clone()));

    info!("{:?}", ds.query_all_balance());

    let tpc = TransactionProcessContext {
        relay: ttc.relay.clone(),
        tx_process: build_runtime(10, "test_tx_proc"),
    };

    let (dbl1, dbl2) = ttc.tx_gen.generate_double_spend_tx();

    let res = submitter.spawn({
        let context = tpc.clone();
        let transaction = dbl1.clone().transaction;
        async move { context.process(&transaction).await }
    });
    let res2 = submitter.spawn({
        let context = tpc.clone();
        let transaction = dbl2.clone().transaction;
        async move { context.process(&transaction).await }
    });
    let r1 = submitter.block_on(res).unwrap();
    let r2 = submitter.block_on(res2).unwrap();

    info!("{:?}", r1);
    info!("{:?}", r2);
    info!("{:?}", ds.query_all_balance());

    let results = vec![r1, r2];
    assert!(results.iter().find(|x| x.is_err()).is_some());
    assert!(results.iter().find(|x| x.is_ok()).is_some());
}

// https://docs.rs/crossbeam-channel/0.5.1/crossbeam_channel/index.html
// select timeout after duration
// try iter
// try non blocking async
