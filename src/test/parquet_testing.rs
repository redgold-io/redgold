use crate::core::relay::Relay;
use redgold_data::parquet_export::ParquetExporter;
use std::path::PathBuf;
#[ignore]
#[tokio::test]
async fn debug_parquet() {
    let r = Relay::dev_default().await;
    let res = r.ds.parquet_export_archive_historical_tx(
        &PathBuf::from("test-parquet-export"),
    ).await.expect("parquet");


}