use std::path::PathBuf;
use async_trait::async_trait;
use tracing::{error, info};
use redgold_data::data_store::DataStore;
use redgold_data::parquet_export::ParquetExporter;
use redgold_schema::{ErrorInfoContext, RgResult};
use crate::core::relay::Relay;
use redgold_common_no_wasm::stream_handlers::IntervalFold;
use aws_sdk_s3 as s3;
use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{Delete, ObjectIdentifier, StorageClass};
use itertools::Itertools;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReadDirStream;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::config_data::ConfigData;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::observability::errors::{EnhanceErrorInfo, Loggable};
use redgold_schema::structs::ErrorInfo;
use redgold_schema::util::times::{ToMillisFromTimeString, ToTimeString};
use crate::util;


#[async_trait]
pub trait S3Constructor {
    async fn s3_client(&self) -> s3::Client;
}

#[async_trait]
impl S3Constructor for ConfigData {
    async fn s3_client(&self) -> s3::Client {
        if let Some(k) = self.keys.as_ref() {
            if let Some(a) = k.aws_access.as_ref() {
                if let Some(s) = k.aws_secret.as_ref() {
                    let config = aws_config::from_env()
                        .region("us-west-1")
                        .credentials_provider(aws_sdk_s3::config::Credentials::new(
                            a,
                            s,
                            None,
                            None,
                            "static"
                        ))
                        .load()
                        .await;
                        return s3::Client::new(&config);
                }
            }
        }
        let config = aws_config::from_env()
            .region("us-west-1")
            .load()
            .await;
        s3::Client::new(&config)
    }
}

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
        if let (Some(bucket), server_index) = 
            (self.relay.node_config.s3_backup(), 
             self.relay.node_config.server_index()) {
            return self.s3_ls(bucket, "".to_string()).await.is_ok()
        }
        false
    }

    pub async fn backup_s3(&self) -> RgResult<()> {
        let ct = util::current_time_unix() as i64;

        if let (Some(bucket), server_index) =
            (self.relay.node_config.s3_backup(),
             self.relay.node_config.server_index()) {

            let daily_prefix = format!("daily/{}/{}", self.relay.node_config.network.to_std_string(), server_index);
            // let weekly_prefix = format!("weekly/{}", server_index);
            // let monthly_prefix = format!("monthly/{}", server_index);
            info!("Listing keys in bucket {}", bucket);
            let daily_keys = self.s3_ls(bucket, daily_prefix.clone()).await
                .log_error().unwrap_or(vec![]);
            if Self::has_recent_daily_backup(&daily_keys) {
                info!("Skipping backup, not enough time has passed since last backup");
                return Ok(());
            }

            // if daily_keys.len() >= 7 {
            //     if let Some(oldest) = daily_keys.iter().min()
            //         .and_then(|k| k.split('/').last().clone())
            //         .and_then(|k| k.parse::<i64>().ok()) {
            //         let oldest = format!("{}/{}", daily_prefix.clone(), oldest);
            //         // let oldest_to = format!("{}/{}", weekly_prefix, oldest);
            //         // Self::s3_cp(bucket, oldest, ).await?;
            //         self.s3_rm(bucket, oldest).await?;
            //     }
            // }
            let daily_key = format!("{}/{}", daily_prefix.clone(), ct.to_time_string_shorter_underscores());
            let parquet_exports = format!("{}/{}", daily_key, "parquet_exports");
            self.s3_upload_directory(&self.relay.node_config.env_data_folder().parquet_exports(), bucket.clone(), parquet_exports).await?;
        } else {
            info!("No s3_backup_bucket or server_index set")
        };
        Ok(())
    }

    fn has_recent_daily_backup(daily_keys: &Vec<String>) -> bool {
        let ct = util::current_time_unix() as i64;
        if !daily_keys.is_empty() {
            if let Some(o) = daily_keys.iter().max() {
                if let Some(n) = o.split('/').last() {
                    let ms = n.to_string().to_millis_from_time_string_shorter_underscores();
                    if let Some(p) = ms {
                        if ct - p < (86400 / 2) {
                            info!("Not enough time has passed since last backup");
                            return true;
                        }
                    } else {
                        error!("Failed to parse last value of max daily key");
                    }
                } else {
                    error!("Failed to get max daily key last value split /");
                }
            } else {
                error!("Failed to get max daily key");
            }
        }
        false
    }

    pub async fn s3_upload_directory(&self, dir: &PathBuf, bucket: String, prefix: String) -> RgResult<()> {
        let client = self.relay.node_config.config_data.s3_client().await;

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
                } else {
                    file_paths.push(path);
                }
            }
        }

        info!("Upload S3 dir: {:?}", file_paths.iter().map(|p| p.to_string_lossy()).collect_vec());

        for path in file_paths {
            let key = path.strip_prefix(dir).unwrap().to_string_lossy().to_string();
            let key = format!("{}/{}", prefix.trim_end_matches('/'), key);
            let mime = mime_guess::from_path(&path)
                .first_or_octet_stream()
                .to_string();
            let file_size = tokio::fs::metadata(&path).await.error_info("Failed to read file metadata")?.len();

            let body = ByteStream::from_path(&path).await.error_info("Failed to read file")?;
                client.put_object()
                    .bucket(&bucket)
                    .key(&key)
                    .body(body)
                    .content_type(mime.clone())
                    .send()
                    .await

                    .error_info("S3 put object failure")
                    .with_detail("key", key)
                    .with_detail("bucket", bucket.clone())
                    .with_detail("path", path.to_string_lossy().to_string())
                    .with_detail("prefix", prefix.clone())
                    .with_detail("mime", mime)
                    .with_detail("file_size", file_size.to_string())
                    ?;
        }

        Ok(())
    }

    async fn s3_ls(&self, bucket: &String, prefix: String) -> Result<Vec<String>, ErrorInfo> {
        let client = self.relay.node_config.config_data.s3_client().await;
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

    async fn s3_cp(&self, bucket: &String, prefix_source: String, prefix_destination: String) -> Result<(), ErrorInfo> {

        let client = self.relay.node_config.config_data.s3_client().await;
        // List all objects with prefix_source
        let keys = self.s3_ls(bucket, prefix_source.clone()).await?;

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
    async fn s3_rm(&self, bucket: &String, prefix: String) -> Result<(), ErrorInfo> {

        let client = self.relay.node_config.config_data.s3_client().await;
        let keys = self.s3_ls(bucket, prefix.clone()).await?;

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
        if self.relay.node_config.aws_access().is_some() {
            self.do_backup().await.log_error().ok();
        }
        Ok(())
    }
}

#[ignore]
#[tokio::test]
async fn test_aws_backup() {
    // AwsBackup::s3_upload_directory(&PathBuf::from("./testdir"), "redgold-backups".to_string(), "testdir3".to_string()).await.unwrap();
    // let res = AwsBackup::s3_ls(&"redgold-backups".to_string(), "testdir/testdir".to_string()).await.unwrap();
    // println!("{:?}", res);
}