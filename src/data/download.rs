use crate::core::internal_message::{new_channel, RecvAsyncErrorInfo, SendErrorInfo};
use crate::core::internal_message::PeerMessage;
use crate::core::relay::Relay;
use redgold_data::data_store::DataStore;
use crate::genesis::create_test_genesis_transaction;
use crate::schema::structs::{
    DownloadDataType, DownloadRequest, DownloadResponse, NodeState, Request, Response,
};
use crate::util;
use log::{error, info};
use redgold_schema::constants::EARLIEST_TIME;
use std::time::Duration;
use metrics::{counter, gauge};
use tokio_stream::Elapsed;
use redgold_schema::{ProtoHashable, RgResult, SafeOption, structs, WithMetadataHashable};
use redgold_schema::structs::{ErrorInfo, UtxoId};
use redgold_schema::EasyJson;
use crate::observability::logging::Loggable;
use crate::observability::metrics_help::WithMetrics;

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
    });
    // TODO: Handle retries to other peers
    let response = relay.send_message_sync(request, key, None).await?;
    response.as_error_info()?;
    response.download_response.ok_msg("Missing download response")
}

pub async fn download_all(
    relay: &Relay,
    start_time: i64,
    end_time: i64,
    key: &structs::PublicKey,
) -> Result<bool, ErrorInfo> {

    let mut got_data = false;

    if let Ok(dr) = download_msg(
        &relay,
        start_time,
        end_time,
        DownloadDataType::UtxoEntry,
        key.clone(),
    ).await.log_error().with_err_count("redgold.download.utxo_error") {
        // TODO: Change this to include peer observations as well to determine if it's sufficient to accept.
        let utxo_entries = dr.utxo_entries;
        counter!("redgold.download.utxo").increment(utxo_entries.len() as u64);
        for utxo in utxo_entries {
            if let Some(utxo_id) = utxo.utxo_id.as_ref() {
                got_data = true;
                if !relay.utxo_channels.contains_key(utxo_id) {
                    relay.ds.transaction_store.insert_utxo(&utxo).await?;
                }
            }
        }
    }

    if let Ok(dr) = download_msg(
        &relay,
        start_time,
        end_time,
        DownloadDataType::TransactionEntry,
        key.clone(),
    ).await.log_error().with_err_count("redgold.download.transaction_error") {
        // TODO: Change this to include peer observations as well to determine if it's sufficient to accept.
        let txs = dr.transactions;
        counter!("redgold.download.transaction").increment(txs.len() as u64);
        for txe in txs {
            if let Some(tx) = txe.transaction.as_ref() {
                got_data = true;
                if !relay.transaction_known(&tx.calculate_hash()).await? {
                    relay
                        .ds
                        .transaction_store
                        .insert_transaction(&tx, txe.time as i64, true, None, false)
                        .await?;
                }
            }
        }
    }

    if let Ok(dr) = download_msg(
        &relay,
        start_time,
        end_time,
        DownloadDataType::ObservationEntry,
        key.clone(),
    ).await.log_error().with_err_count("redgold.download.observation_error") {
        // TODO: Change this to include peer observations as well to determine if it's sufficient to accept.
        let obes = dr.observations;
        counter!("redgold.download.observation").increment(obes.len() as u64);
        for obe in obes {
            got_data = true;
            if let Some(tx) = obe.observation.as_ref() {
                relay.ds.observation.insert_observation_and_edges(tx).await?;
            }
        }
    }

    if let Ok(dr) = download_msg(
        &relay,
        start_time,
        end_time,
        DownloadDataType::ObservationEdgeEntry,
        key.clone(),
    ).await.log_error().with_err_count("redgold.download.observation_edge_error") {
        // TODO: Change this to include peer observations as well to determine if it's sufficient to accept.
        let obes = dr.observation_edges;
        counter!("redgold.download.observation").increment(obes.len() as u64);
        for obe in obes {
            got_data = true;
            relay.ds.observation.insert_observation_edge(&obe).await?;
        }
    }
    Ok(got_data)
}

struct PerfTimer {
    start: std::time::Instant,
    latest: std::time::Instant,
    map: std::collections::HashMap<String, i64>,
}

impl PerfTimer {
    pub fn new() -> Self {
        let start = std::time::Instant::now();
        Self {
            start,
            latest: start,
            map: Default::default(),
        }
    }
    pub fn mark(&mut self) {
        self.latest = std::time::Instant::now();
    }

    pub fn record(&mut self, name: impl Into<String>) {
        let millis = self.millis();
        self.map.insert(name.into(), millis);
    }

    pub fn millis(&mut self) -> i64 {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.latest);
        self.latest = now;
        let millis = elapsed.as_millis() as i64;
        millis
    }
}

/**
Current 'direct trusted' download process to bootstrap historical data.
Should not interfere with other live synchronization processes. Requests backwards in time

Actual future download process should start with getting the current parquet part file
snapshot through IPFS for all the different data types. The compacted format.
*/
pub async fn download(relay: Relay, bootstrap_pks: Vec<structs::PublicKey>) -> RgResult<()> {


    let recent = relay.ds.transaction_store.query_recent_transactions(Some(1), None).await?;
    let min_time = recent.iter().filter_map(|t| t.time().ok()).min().cloned().unwrap_or(EARLIEST_TIME);
    let start_time = util::current_time_millis_i64();

    // Time slice by days backwards.
    let mut no_data_count = 0;

    // TODO: Not this, also a maximum earliest lookback period.
    let bootstrap = bootstrap_pks.get(0).expect("bootstrap").clone();

    let mut cur_end = start_time;

    let mut perf_timer = PerfTimer::new();

    while no_data_count < 3 && cur_end > min_time{
        let prev_day = cur_end - 1000 * 60 * 60 * 24;

        let got_data = download_all(&relay, prev_day, cur_end, &bootstrap).await?;

        if got_data {
            no_data_count = 0;
        } else {
            no_data_count += 1;
        }
        cur_end = prev_day;
    }

    let secs = perf_timer.millis() / 1000;
    gauge!("redgold.download.time_seconds").set(secs as f64);
    info!("Download time seconds {}", secs);

    Ok(())
}


pub async fn process_download_request(
    relay: &Relay,
    download_request: DownloadRequest,
) -> RgResult<DownloadResponse> {
    counter!("redgold.download.request").increment(1);
    Ok(DownloadResponse {
        utxo_entries: {
            if download_request.data_type != DownloadDataType::UtxoEntry as i32 {
                vec![]
            } else {
                relay
                    .ds
                    .transaction_store
                    .utxo_filter_time(download_request.start_time as i64, download_request.end_time as i64).await
                    ?
            }
        },
        transactions: {
            if download_request.data_type != DownloadDataType::TransactionEntry as i32 {
                vec![]
            } else {
                relay.ds.transaction_store.query_time_transaction(
                    download_request.start_time as i64,
                    download_request.end_time as i64,
                ).await?
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
    })
}
