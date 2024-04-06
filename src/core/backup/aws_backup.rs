use async_trait::async_trait;
use log::info;
use redgold_schema::RgResult;
use crate::core::relay::Relay;
use crate::core::stream_handlers::IntervalFold;

pub struct AwsBackup {
    pub relay: Relay,
}

impl AwsBackup {
    pub fn new(relay: &Relay) -> AwsBackup {
        AwsBackup {
            relay: relay.clone()
        }
    }
}

#[async_trait]
impl IntervalFold for AwsBackup {
    async fn interval_fold(&mut self) -> RgResult<()> {
        if self.relay.node_config.opts.aws_access_key_id.is_some() {
           info!("AWS Backup started");
        }
        Ok(())
    }
}