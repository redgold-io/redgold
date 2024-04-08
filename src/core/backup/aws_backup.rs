use std::path::PathBuf;
use async_trait::async_trait;
use log::info;
use redgold_data::data_store::DataStore;
use redgold_data::parquet_export::ParquetExporter;
use redgold_schema::{ErrorInfoContext, RgResult, WithMetadataHashable};
use crate::core::relay::Relay;
use crate::core::stream_handlers::IntervalFold;
use aws_sdk_s3 as s3;
use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{Delete, ObjectIdentifier, StorageClass};
use itertools::Itertools;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReadDirStream;
use redgold_schema::observability::errors::{EnhanceErrorInfo, Loggable};
use redgold_schema::structs::ErrorInfo;
use crate::util;

pub struct AwsBackup {
    pub relay: Relay,
}

impl AwsBackup {
    pub fn new(relay: &Relay) -> AwsBackup {
        AwsBackup {
            relay: relay.clone()
        }
    }

    pub async fn can_do_backup(&self) -> bool {
        if let (Some(bucket), Some(server_index)) =
            (self.relay.node_config.opts.s3_backup_bucket.as_ref(),
             self.relay.node_config.opts.server_index.as_ref()) {
            return Self::s3_ls(bucket, "".to_string()).await.is_ok()
        }
        false
    }

    pub async fn backup_s3(&self) -> RgResult<()> {
        let ct = util::current_time_unix() as i64;

        if let (Some(bucket), Some(server_index)) =
            (self.relay.node_config.opts.s3_backup_bucket.as_ref(),
             self.relay.node_config.opts.server_index.as_ref()){

            let daily_prefix = format!("daily/{}", server_index);
            // let weekly_prefix = format!("weekly/{}", server_index);
            // let monthly_prefix = format!("monthly/{}", server_index);
            info!("Listing keys in bucket {}", bucket);
            let daily_keys = Self::s3_ls(bucket, daily_prefix.clone()).await?;
            if !daily_keys.is_empty() {
                let newest = daily_keys.iter().max().unwrap();
                let newest = newest.split('/').last().unwrap();
                let newest = newest.parse::<i64>().unwrap();
                if ct - newest < (86400 / 2) {
                    info!("Not enough time has passed since last backup");
                    return Ok(());
                }
            }
            if daily_keys.len() >= 7 {
                let oldest = daily_keys.iter().min().unwrap();
                let oldest = oldest.split('/').last().unwrap();
                let oldest = oldest.parse::<i64>().unwrap();
                let oldest = format!("{}/{}", daily_prefix.clone(), oldest);
                // let oldest_to = format!("{}/{}", weekly_prefix, oldest);
                // Self::s3_cp(bucket, oldest, ).await?;
                Self::s3_rm(bucket, oldest).await?;
            }
            let daily_key = format!("{}/{}", daily_prefix.clone(), ct);
            let parquet_exports = format!("{}/{}", daily_key, "parquet_exports");
            Self::s3_upload_directory(&self.relay.node_config.env_data_folder().parquet_exports(), bucket.clone(), parquet_exports).await?;
            // let weekly_keys = Self::s3_ls(bucket, weekly_prefix).await?;
        };
        Ok(())

    }


    async fn s3_upload_directory(dir: &PathBuf, bucket: String, prefix: String) -> RgResult<()> {
        let config = aws_config::load_from_env().await;
        let client = s3::Client::new(&config);


        let mut file_paths = Vec::new();
        let mut dirs = vec![dir.clone()];

        while let Some(dir) = dirs.pop() {
            let mut entries = ReadDirStream::new(tokio::fs::read_dir(dir).await
                .error_info("Failed to read directory")?
            );
            while let Some(entry) = entries.next().await {
                let entry = entry.error_info("Bad read")?;
                let path = entry.path();

                if path.is_dir() {
                    dirs.push(path.clone());
                }
                file_paths.push(path);
            }
        }

        for path in file_paths {
            let key = path.strip_prefix(dir).unwrap().to_string_lossy().to_string();
            let key = format!("{}/{}", prefix, key);
            let body = ByteStream::from_path(&path).await.error_info("Failed to read file")?;
                client.put_object()
                    .bucket(&bucket)
                    .key(&key)
                    .body(body)
                    .send()
                    .await
                    .error_info("S3 put object failure")?;
        }

        Ok(())
    }
    async fn s3_ls(bucket: &String, prefix: String) -> Result<Vec<String>, ErrorInfo> {
        let config = ::aws_config::load_from_env().await;
        let client = s3::Client::new(&config);
        let ls_result = client.list_objects().bucket(bucket)
            .prefix(prefix.clone())
            .send().await.error_info("List failed").add(prefix.clone())?;
        let keys = ls_result.contents().iter().flat_map(|content| {
            let content_vec = content.to_vec();
            let keys = content_vec.iter()
                .flat_map(|s3_object| s3_object.key())
                .map(|s| s.to_string())
                .collect_vec();
            keys
        }).collect_vec();
        Ok(keys)
    }

    async fn s3_cp(bucket: &String, prefix_source: String, prefix_destination: String) -> Result<(), ErrorInfo> {
        let config = ::aws_config::load_from_env().await;
        let client = Client::new(&config);

        // List all objects with prefix_source
        let keys = Self::s3_ls(bucket, prefix_source.clone()).await?;

        // Copy each object to prefix_destination
        for key in keys {
            let source_key = key.clone();
            let destination_key = key.replace(&prefix_source, &prefix_destination);

            let copy_result = client
                .copy_object()
                .copy_source(format!("{}/{}", bucket, source_key))
                .bucket(bucket)
                .storage_class(StorageClass::Glacier)
                .key(&destination_key)
                .send()
                .await
                .error_info("Copy failed")
                .add(format!("Source: {}, Destination: {}", source_key, destination_key))?;

            info!("Copied: {} -> {}", source_key, destination_key);
        }

        Ok(())
    }
    async fn s3_rm(bucket: &String, prefix: String) -> Result<(), ErrorInfo> {
        let config = ::aws_config::load_from_env().await;
        let client = Client::new(&config);
        let keys = Self::s3_ls(bucket, prefix.clone()).await?;

        if keys.is_empty() {
            info!("No objects found with prefix: {}", prefix);
            return Ok(());
        }

        let delete_objects = keys
            .iter()
            .map(|key| ObjectIdentifier::builder().key(key).build())
            .collect_vec();

        let delete_result = client
            .delete_objects()
            .bucket(bucket)
            .delete(Delete::builder().set_objects(Some(delete_objects)).build())
            .send()
            .await
            .error_info("Delete failed")
            .add(prefix.clone())?;

        info!("Deleted {} objects with prefix: {}", keys.len(), prefix);

        Ok(())
    }

    async fn do_backup(&mut self) -> Result<(), ErrorInfo> {
        info!("AWS Backup started");
        // This part is just the parquet export, which can be moved elsewhere.
        let folder = self.relay.node_config.env_data_folder();
        let dir = folder.parquet_exports();
        tokio::fs::remove_dir(&dir).await.ok();
        tokio::fs::create_dir_all(&dir).await.error_info("Couldn't create parquet export dir")?;
        self.relay.ds
            .parquet_export_archive_historical_tx(&folder.parquet_tx())
            .await?;
        let mut self_obs = self.relay.ds.observation.get_pk_observations(&self.relay.node_config.public_key(), 1e9 as i64).await?;
        self_obs.sort_by(|a, b| a.time().expect("").cmp(&b.time().expect("")));
        let parquet_self_obs = folder.parquet_self_observations();
        tokio::fs::create_dir_all(&parquet_self_obs).await.error_info("Couldn't create parquet export dir")?;
        DataStore::write_transactions_partitioned(&parquet_self_obs, self_obs)?;
        self.backup_s3().await?;
        info!("AWS Backup finished");
        Ok(())
    }
}

#[async_trait]
impl IntervalFold for AwsBackup {
    async fn interval_fold(&mut self) -> RgResult<()> {
        if self.relay.node_config.opts.aws_access_key_id.is_some() {
            self.do_backup().await.log_error().ok();
        }
        Ok(())
    }
}