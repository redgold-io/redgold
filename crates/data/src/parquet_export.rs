use crate::data_store::DataStore;
use async_trait::async_trait;
use itertools::Itertools;
use log::info;
use polars::export::chrono::{DateTime, Utc};
use polars::frame::row::Row;
use redgold_schema::structs::Transaction;
use redgold_schema::{error_info, util, ErrorInfoContext, RgResult};
use std::fs::File;
use std::path::PathBuf;

#[async_trait]
pub trait ParquetExporter {
    async fn get_time_ordered_transactions(&self) -> RgResult<Vec<Transaction>>;
    async fn parquet_export_archive_historical_tx(&self, tx_path: &PathBuf) -> RgResult<()>;
    // async fn parquet_export_archive_historical_ym(&self, path: &PathBuf) -> RgResult<()>;
}

#[async_trait]
impl ParquetExporter for DataStore {
    async fn get_time_ordered_transactions(&self) -> RgResult<Vec<Transaction>> {

        info!("Getting time-ordered transactions");
        let txs =
            self.transaction_store.query_time_transaction_accepted_ordered(0, times::current_time_millis()).await?;
        //
        // info!("Getting time-ordered observations");
        // let obs = self.observation.query_time_observation(0, util::current_time_millis()).await?
        //     .into_iter().flat_map(|o| o.observation).collect_vec();
        //
        // info!("Got {} transactions and {} observations", txs.len(), obs.len());
        // let mut all = vec![];
        // all.extend(txs);
        // all.extend(obs);
        // let mut all2 = all.into_iter()
        //     .flat_map(|a| a.time().map(|t| (t.clone(), a.clone())))
        //     .collect_vec();
        // info!("started sorting");
        //
        // all2.sort_by(|a, b| a.0.cmp(&b.0));
        // info!("Finished sorting");
        // Ok(all2)
        Ok(txs)
    }
    async fn parquet_export_archive_historical_tx(&self, tx_path: &PathBuf) -> RgResult<()> {
        std::fs::remove_dir(tx_path).ok();
        std::fs::create_dir_all(tx_path).error_info("Failed to create directory for Parquet export")?;
        let all2 = self.get_time_ordered_transactions().await?;
        Self::write_transactions_partitioned(tx_path, all2)?;

        Ok(())
    }
    // async fn parquet_export_archive_historical_ym(&self, path: &PathBuf) -> RgResult<()> {
    //     let all2 = self.get_time_ordered_transactions().await?;
    //
    //     let mut cur_ym: Option<String> = None;
    //
    //     let mut buf = Vec::new();
    //     let max_byte_count = 128 * 1024 * 1024; // 128 MiB
    //     let mut byte_count = 0;
    //     let mut part = 0;
    //
    //     for (tx) in all2 {
    //         let time = tx.clone().time()?.clone();
    //         let year_month = millis_to_year_month(time);
    //         if cur_ym.is_none() {
    //             cur_ym = Some(year_month.clone());
    //         }
    //         let ser = tx.proto_serialize();
    //         let ym_change = cur_ym != Some(year_month.clone());
    //         if byte_count + ser.len() > max_byte_count || ym_change {
    //             let path_ym = path.join(cur_ym.clone().unwrap());
    //             let part_fnm = index_to_part_file(part);
    //             let path_file = path_ym.join(part_fnm);
    //             write_parquet_file(&path_file, &buf)?;
    //             buf.clear();
    //             byte_count = 0;
    //             part += 1;
    //         }
    //         if ym_change {
    //             cur_ym = Some(year_month);
    //             part = 0;
    //         }
    //         buf.push(tx);
    //     }
    //
    //     Ok(())
    // }
}

impl DataStore {
    fn write_part(path: &PathBuf, buf: &mut Vec<Transaction>, part: i32) -> RgResult<()> {
        let part_fnm = index_to_part_file(part);
        let path_file = path.join(part_fnm);
        write_parquet_file(&path_file, &buf)?;
        Ok(())
    }

    pub fn write_transactions_partitioned(tx_path: &PathBuf, all2: Vec<Transaction>) -> RgResult<()> {
        let mut buf = Vec::new();
        let max_byte_count = 128 * 1024 * 1024; // 128 MiB default
        let mut byte_count = 0;
        let mut part = 0;

        // info!("Starting export");
        for (i, tx) in all2.into_iter().enumerate() {
            let ser = tx.proto_serialize();
            // let mut write_condition = false;
            // if i % 2000 == 0 && i > 0 {
            //     info!("Exporting transaction {}", i);
            //     write_condition = true;
            // }
            let write_condition = byte_count + ser.len() > max_byte_count;
            if write_condition {
                info!("Starting write part {}", part);
                Self::write_part(tx_path, &mut buf, part)?;
                buf.clear();
                byte_count = 0;
                part += 1;
            }
            buf.push(tx);
            byte_count += ser.len();
        }
        if !buf.is_empty() {
            Self::write_part(tx_path, &mut buf, part)?;
        }
        Ok(())
    }
}


use crate::parquet_min_index::{transaction_simple_parquet_schema, translate_tx_simple};
use polars::prelude::*;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::util::timers::PerfTimer;
use redgold_schema::util::times;
use redgold_schema::util::times::current_time_millis;


// Assumes transactions are already sorted by time.
fn write_parquet_file(path: &PathBuf, transactions: &Vec<Transaction>) -> RgResult<()> {

    let mut df = as_dataframe(transactions, Some(current_time_millis()))?;


    let df_shape = df.shape();
    info!("Converted {} transactions out of shape {} {} to DataFrame", df_shape.0, df_shape.1, transactions.len());

    let pt = PerfTimer::named("write_parquet_file");
    info!("finished converting dataframe, Writing Parquet file to {}", path.display());
    let file = File::create(path).error_info("Failed to create Parquet file")?;
    let written = ParquetWriter::new(file)
        .with_statistics(true)
        .with_compression(ParquetCompression::Snappy)
        // .with_data_page_size()
        // .with_row_group_size()
        .finish(&mut df)
        .error_info("Failed to write DataFrame to Parquet")?;
    info!("Wrote {} bytes to Parquet", written);
    //
    // if (written as usize) != transactions.len() {
    //     Err(error_info("Failed to write all rows to Parquet"))
    //         .add(format!("Wrote {} rows out of {}", written, transactions.len()))?;
    // }
    pt.log();
    Ok(())
}

fn as_dataframe(transactions: &Vec<Transaction>, schema_time: Option<i64>) -> RgResult<DataFrame> {
    let schema = transaction_simple_parquet_schema(None);
    info!("Converting {} transactions to rows", transactions.len());
    let t = PerfTimer::named("as_dataframe_translate_tx");
    let rows = transactions.iter().map(|tx| translate_tx_simple(tx)).collect::<RgResult<Vec<Row>>>()?;
    t.log();
    info!("Converted {} transactions to rows", rows.len());
    let t = PerfTimer::named("dataframe_from");
    let df = DataFrame::from_rows_and_schema(&rows, &schema).error_info("Failed to create DataFrame")?;
    t.log();
    info!("Created DataFrame with shape {} {}", df.shape().0, df.shape().1);
    Ok(df)
}

fn index_to_part_file(index: i32) -> String {
    format!("part-{:05}.parquet", index)
}

fn millis_to_year_month(millis: i64) -> String {
    let datetime = DateTime::<Utc>::from_utc(
        chrono::NaiveDateTime::from_timestamp_millis(millis).unwrap(),
        Utc,
    );
    datetime.format("%Y_%m").to_string()
}