use std::collections::{HashMap, HashSet};
use crate::core::internal_message::PeerMessage;
use crate::core::relay::Relay;
use redgold_data::data_store::DataStore;
// use crate::genesis::create_test_genesis_transaction;
use crate::schema::structs::{
    DownloadDataType, DownloadRequest, DownloadResponse, NodeState, Request, Response,
};
use crate::util;
use tracing::{error, info};
use redgold_schema::constants::EARLIEST_TIME;
use std::time::Duration;
use futures::StreamExt;
use itertools::Itertools;
use metrics::{counter, gauge};
use tokio_stream::Elapsed;
use redgold_common::flume_send_help::{new_channel, RecvAsyncErrorInfo, SendErrorInfo};
use redgold_schema::{structs, RgResult, SafeOption};
use redgold_schema::structs::{ErrorInfo, Hash, PublicKey, Transaction, TransactionEntry, UtxoId};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::util::xor_distance::XorDistancePartitionInfo;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::proto_serde::ProtoHashable;
use crate::observability::metrics_help::WithMetrics;
use redgold_schema::structs::BatchTransactionResolveRequest;
use redgold_schema::util::timers::PerfTimer;

#[derive(Clone, Debug)]
pub struct DownloadMaxTimes {
    pub utxo: i64,
    pub transaction: i64,
    pub observation: i64,
    pub observation_edge: i64,
}

pub async fn download_msg(
    relay: &Relay,
    start_time: i64,
    end_time: i64,
    data_type: DownloadDataType,
    key: structs::PublicKey,
) -> RgResult<DownloadResponse> {

    let mut request = Request::empty();
    request.download_request = Some(DownloadRequest {
            start_time: start_time as u64,
            end_time: end_time as u64,
            data_type: data_type as i32,
            offset: None,
            partition_info: relay.partition_info().await?
    });
    // TODO: Handle retries to other peers
    let response = relay.send_message_await_response(request, key, None).await?;
    response.as_error_info()?;
    response.download_response.ok_msg("Missing download response")
}
//
// pub async fn download_all(
//     relay: &Relay,
//     start_time: i64,
//     end_time: i64,
//     key: &structs::PublicKey,
// ) -> Result<bool, ErrorInfo> {
//
//     let mut got_data = false;
//
//     if let Ok(dr) = download_msg(
//         &relay,
//         start_time,
//         end_time,
//         DownloadDataType::UtxoEntry,
//         key.clone(),
//     ).await.log_error().with_err_count("redgold.download.utxo_error") {
//         // TODO: Change this to include peer observations as well to determine if it's sufficient to accept.
//         let utxo_entries = dr.utxo_entries;
//         counter!("redgold.download.utxo").increment(utxo_entries.len() as u64);
//         for utxo in utxo_entries {
//             if let Some(utxo_id) = utxo.utxo_id.as_ref() {
//                 got_data = true;
//                 if !relay.utxo_channels.contains_key(utxo_id) {
//                     relay.ds.transaction_store.insert_utxo(&utxo, None).await.with_err_count("redgold.download.utxo_insert_error").ok();
//                 }
//             }
//         }
//     }
//
//     if let Ok(dr) = download_msg(
//         &relay,
//         start_time,
//         end_time,
//         DownloadDataType::TransactionEntry,
//         key.clone(),
//     ).await.log_error().with_err_count("redgold.download.transaction_error") {
//         // TODO: Change this to include peer observations as well to determine if it's sufficient to accept.
//         let txs = dr.transactions;
//         counter!("redgold.download.transaction").increment(txs.len() as u64);
//         for txe in txs {
//             if let Some(tx) = txe.transaction.as_ref() {
//                 got_data = true;
//                 if !relay.transaction_known(&tx.calculate_hash()).await? {
//                     relay
//                         .write_transaction(&tx, txe.time as i64, None, true)
//                         .await?;
//                 }
//             }
//         }
//     }
//
//     if let Ok(dr) = download_msg(
//         &relay,
//         start_time,
//         end_time,
//         DownloadDataType::ObservationEntry,
//         key.clone(),
//     ).await.log_error().with_err_count("redgold.download.observation_error") {
//         // TODO: Change this to include peer observations as well to determine if it's sufficient to accept.
//         let obes = dr.observations;
//         counter!("redgold.download.observation").increment(obes.len() as u64);
//         for obe in obes {
//             got_data = true;
//             if let Some(tx) = obe.observation.as_ref() {
//                 relay.ds.observation.insert_observation_and_edges(tx).await
//                     .with_err_count("redgold.download.observation_insert_error").ok();;
//             }
//         }
//     }
//
//     if let Ok(dr) = download_msg(
//         &relay,
//         start_time,
//         end_time,
//         DownloadDataType::ObservationEdgeEntry,
//         key.clone(),
//     ).await.log_error().with_err_count("redgold.download.observation_edge_error") {
//         // TODO: Change this to include peer observations as well to determine if it's sufficient to accept.
//         let obes = dr.observation_edges;
//         counter!("redgold.download.observation").increment(obes.len() as u64);
//         for obe in obes {
//             got_data = true;
//             relay.ds.observation.insert_observation_edge(&obe).await
//                 .with_err_count("redgold.download.oe_insert_error").ok();;
//         }
//     }
//     Ok(got_data)
// }

/**
Current 'direct trusted' download process to bootstrap historical data.
Should not interfere with other live synchronization processes. Requests backwards in time

Actual future download process should start with getting the current parquet part file
snapshot through IPFS for all the different data types. The compacted format.
*/
pub async fn download(relay: Relay, bootstrap_pks: Vec<structs::PublicKey>) -> RgResult<()> {
    let mut perf_timer = PerfTimer::new();

    // First bootstrap off UTXO set within peer distance
    let start_time = util::current_time_millis_i64();

    let utxo_hashes = get_all_hashes(
        &relay, bootstrap_pks.clone(), start_time, DownloadDataType::UtxoHash
    ).await?;

    gauge!("redgold_download_utxo_hashes", &relay.node_config.gauge_id()).set(utxo_hashes.len() as f64);

    // TODO: FP

    let missing_utxo_tx_hashes = filter_known_hashes(&relay, &utxo_hashes).await?;

    gauge!("redgold_download_utxo_missing_hashes", &relay.node_config.gauge_id()).set(missing_utxo_tx_hashes.len() as f64);

    // Live UTXO transactions merged
    batch_resolve_txs(&relay, missing_utxo_tx_hashes, &bootstrap_pks, false).await?;


    // Historical Transactions -- todo: truncate time based on disk space
    let historical_tx_hashes = get_all_hashes(
        &relay, bootstrap_pks.clone(), start_time, DownloadDataType::TransactionHash
    ).await?;

    gauge!("redgold_download_tx_hashes", &relay.node_config.gauge_id()).set(historical_tx_hashes.len() as f64);

    let missing_historical_tx_hashes = filter_known_hashes(&relay, &historical_tx_hashes).await?;

    gauge!("redgold_download_tx_missing_hashes", &relay.node_config.gauge_id()).set(missing_historical_tx_hashes.len() as f64);

    batch_resolve_txs(&relay, missing_historical_tx_hashes, &bootstrap_pks, false).await?;

    // Historical Observations -- todo: truncate time based on disk space, and/or truncate by live utxo set

    let observation_hashes = get_all_hashes(
        &relay, bootstrap_pks.clone(), start_time, DownloadDataType::ObservationTxHash
    ).await?;

    gauge!("redgold_download_obs_hashes", &relay.node_config.gauge_id()).set(observation_hashes.len() as f64);

    let missing_obs_hashes = filter_known_observation_hashes(&relay, &observation_hashes).await?;

    gauge!("redgold_download_obs_missing_hashes", &relay.node_config.gauge_id()).set(missing_obs_hashes.len() as f64);

    batch_resolve_txs(&relay, missing_obs_hashes, &bootstrap_pks, true).await?;



    // let recent = relay.ds.transaction_store.query_recent_transactions(Some(1), None).await?;
    // let min_time = recent.iter().filter_map(|t| t.time().ok()).min().cloned().unwrap_or(EARLIEST_TIME);

    // Time slice by days backwards.
    let no_data_count = 0;

    // TODO: Not this, also a maximum earliest lookback period.
    let bootstrap = bootstrap_pks.get(0).expect("bootstrap").clone();

    // Force genesis configuration
    if let Some(g_time) = download_genesis(&relay, bootstrap_pks).await? {
        // Workaround to get genesis currently valid UTXOs
        // download_all(&relay, g_time - 1, g_time + 1, &bootstrap).await?;
    }

    //
    // let mut cur_end = start_time;
    //
    // while no_data_count < 3 && cur_end > min_time{
    //     let prev_day = cur_end - 1000 * 60 * 60 * 24;
    //
    //     let got_data = download_all(&relay, prev_day, cur_end, &bootstrap).await?;
    //
    //     if got_data {
    //         no_data_count = 0;
    //     } else {
    //         no_data_count += 1;
    //     }
    //     cur_end = prev_day;
    // }
    //
    let secs = perf_timer.millis() / 1000;
    gauge!("redgold.download.time_seconds", &relay.node_config.gauge_id()).set(secs as f64);
    info!("Download time seconds {}", secs);

    Ok(())
}

async fn filter_known_hashes(relay: &Relay, hashes: &HashSet<Hash>) -> Result<Vec<Hash>, ErrorInfo> {
    let mut missing_utxo_tx_hashes = vec![];

    for hash in hashes {
        let tx = relay.ds.transaction_store.query_maybe_transaction(&hash).await?;
        // TODO: Do we want to overwrite rejections or no?
        if tx.is_none() {
            missing_utxo_tx_hashes.push(hash.clone());
        }
    }
    Ok(missing_utxo_tx_hashes)
}

async fn filter_known_observation_hashes(relay: &Relay, obs_hash: &HashSet<Hash>) -> Result<Vec<Hash>, ErrorInfo> {
    let mut missing = vec![];

    for tx_hash in obs_hash {
        let tx = relay.ds.observation.query_observation(tx_hash).await?;
        if tx.is_none() {
            missing.push(tx_hash.clone());
        }
    }
    Ok(missing)
}

async fn batch_resolve_txs(relay: &Relay, missing_hashes: Vec<Hash>, bootstrap_pks: &Vec<PublicKey>, is_observation: bool)
-> RgResult<()> {

    let mut still_missing = missing_hashes.clone();

    while !still_missing.is_empty() {
        for pk in bootstrap_pks {
            for chunk in missing_hashes.chunks(1000) {
                let success =
                    process_batch_resolve(relay, pk, chunk.to_vec(), is_observation).await;
                if let Ok(s) = success {
                    for tx in s {
                        if let Some(txa) = tx.transaction.as_ref() {
                            let hash = txa.hash_or();
                            still_missing.retain(|h| { h != &hash });
                            if is_observation {
                                relay.ds.observation.insert_observation_and_edges(txa).await?;
                            } else {
                                relay.ds.accept_transaction(
                                    &txa, tx.time as i64, None, true
                                ).await?;
                            }
                        }
                    }
                }
            }
        }
        if still_missing.is_empty() {
            break;
        }
        error!("Still missing txs: {}", still_missing.len());
        error!("Still missing txs: {}", still_missing.json_or());
        return Err(ErrorInfo::error_info("Unable to resolve all transaction hashes during download, ran out of nodes"));
    }


    Ok(())
}

async fn process_batch_resolve(
    relay: &Relay, pk: &PublicKey, chunk: Vec<Hash>,
    is_observation: bool
) -> RgResult<Vec<TransactionEntry>> {
    let mut req = Request::empty();
    let batch_req = BatchTransactionResolveRequest {
        hashes: chunk.clone(),
        is_observation: Some(is_observation)
    };
    req.batch_transaction_resolve_request = Some(batch_req);
    let response = relay.send_message_await_response(req, pk.clone(), None).await?;
    let r = response.batch_transaction_resolve_response.ok_msg("Missing batch response")?;
    let txs: Vec<TransactionEntry> = r.transactions;

    let mut success = vec![];
    for mut tx in txs.into_iter() {
        if let Some(txx) = tx.transaction.as_mut() {
            txx.with_hash();
            let h = txx.hash_or();
            if chunk.contains(&h) {
                success.push(tx);
            }
        }
    }
    Ok(success)
}

pub async fn get_all_hashes(
    r: &Relay, bootstrap_pks: Vec<PublicKey>, start_time: i64, data_type: DownloadDataType
) -> RgResult<HashSet<Hash>> {

    let mut futs = vec![];

    for pk in bootstrap_pks {
        let fut = download_hashes_all_time(r, pk.clone(), start_time, data_type);
        futs.push(fut);
    }

    let res = futures::future::join_all(futs).await;

    let mut all = HashSet::new();
    let pk = r.node_config.public_key();

    let pi = r.partition_info().await?;
    for result in res {
        if let Ok(hashes) = result.log_error() {
            for h in hashes {
                if pi.tx_hash_distance(&h, &pk) {
                    all.insert(h);
                }
            }
        }
    }

    Ok(all)

}

fn has_data(dl_response: &DownloadResponse) -> bool {
    dl_response.utxo_entries.len() > 0 ||
        dl_response.transactions.len() > 0 ||
        dl_response.observations.len() > 0 ||
        dl_response.observation_edges.len() > 0 ||
        dl_response.hashes.len() > 0
}

// TODO: Cutoff times
async fn download_hashes_all_time(
    r: &Relay,
    pk: PublicKey,
    start_time: i64,
    download_data_type: DownloadDataType,
) -> RgResult<HashSet<Hash>> {

    let mut no_data_count = 0;
    let mut cur_end = start_time;

    let mut hashes = HashSet::new();

    while no_data_count < 3 {
        let prev = cur_end - 1000 * 60 * 60 * 24 * 5; // 5 days per request

        let response = download_msg(
            &r, prev, cur_end, download_data_type, pk.clone()
        ).await?;

        let got_data = response.hashes.len() > 0;
        if got_data {
            no_data_count = 0;
            let these_hashes: Vec<Hash> = response.hashes.clone();
            hashes.extend(these_hashes);
        } else {
            no_data_count += 1;
        }
        cur_end = prev;
    }
    Ok(hashes)
}

async fn download_genesis(relay: &Relay, bootstrap_pks: Vec<PublicKey>) -> Result<Option<i64>, ErrorInfo> {
    if relay.ds.config_store.get_genesis().await?.is_none() {
        let mut r = Request::default();
        r.genesis_request = Some(structs::GenesisRequest::default());
        let genesis = relay.broadcast_async(bootstrap_pks.clone(), r, None).await?;
        let hm: HashMap<Hash, (Transaction, i64)> = HashMap::new();
        let filtered = genesis.iter().filter_map(|g| {
            if let Ok(r) = g {
                if let Some(genesis) = &r.genesis_response {
                    return Some(genesis)
                }
            }
            None
        });
        let accum = filtered.fold(hm, |mut acc, g| {
            let h = g.calculate_hash();
            let count = acc.get(&h).map(|(_, c)| c.clone()).unwrap_or(1i64);
            acc.insert(h, (g.clone(), count));
            acc
        });
        let gens = accum.iter().max_by(|(_, (_, c1)), (_, (_, c2))| {
            c1.cmp(&c2)
        });
        let gen_actual = gens.ok_msg("No genesis response")?.1.0.clone();
        relay.ds.config_store.store_genesis(&gen_actual).await?;
        let result = gen_actual.time();
        let g_time = result?.clone();
        if relay.ds.transaction_store.query_maybe_transaction(&gen_actual.calculate_hash()).await?.is_none() {
            relay.write_transaction(&gen_actual, g_time, None, true).await?;
        }
        return Ok(Some(g_time))
    }
    Ok(None)
}


pub async fn process_download_request(
    relay: &Relay,
    download_request: DownloadRequest,
    pk: Option<&PublicKey>
) -> RgResult<DownloadResponse> {
    counter!("redgold.download.request").increment(1);
    let hashes = {
        let dtype = DownloadDataType::from_i32(download_request.data_type);
        if let Some(d) = dtype {
            match d {
                DownloadDataType::UtxoHash => {
                    relay.ds.utxo.utxo_tx_hashes_time(
                        download_request.start_time as i64,
                        download_request.end_time as i64,
                    ).await?
                }
                DownloadDataType::TransactionHash => {
                    relay.ds.transaction_store.accepted_time_tx_hashes(
                        download_request.start_time as i64,
                        download_request.end_time as i64,
                    ).await?
                }
                DownloadDataType::ObservationTxHash => {
                    relay.ds.observation.accepted_time_observation_hashes(
                        download_request.start_time as i64,
                        download_request.end_time as i64,
                    ).await?
                }
                _ => vec![]
            }
        } else {
            vec![]
        }
    };
    Ok(DownloadResponse {
        utxo_entries: {
            if download_request.data_type != DownloadDataType::UtxoEntry as i32 {
                vec![]
            } else {
                relay
                    .ds
                    .utxo
                    .utxo_filter_time(download_request.start_time as i64, download_request.end_time as i64).await?
            }
        },
        transactions: {
            if download_request.data_type != DownloadDataType::TransactionEntry as i32 {
                vec![]
            } else {
                relay.ds.transaction_store.query_time_transaction(
                    download_request.start_time as i64,
                    download_request.end_time as i64,
                ).await?.into_iter().map(|tx| {
                    let time = tx.time().unwrap().clone() as u64;
                    TransactionEntry {
                        transaction: Some(tx),
                        time,
                    }
                }).collect()
            }
        },
        observations: {
            if download_request.data_type != DownloadDataType::ObservationEntry as i32 {
                vec![]
            } else {
                relay.ds.observation.query_time_observation(
                    download_request.start_time as i64,
                    download_request.end_time as i64,
                ).await?
            }
        },
        observation_edges: {
            if download_request.data_type != DownloadDataType::ObservationEdgeEntry as i32 {
                vec![]
            } else {
                relay.ds.observation.query_time_observation_edge(
                    download_request.start_time as i64,
                    download_request.end_time as i64,
                ).await?
            }
        },
        // TODO: not this
        complete_response: true,
        hashes: hashes
    })
}
