use async_trait::async_trait;
use redgold_schema::RgResult;
use crate::core::relay::Relay;
use crate::core::stream_handlers::IntervalFold;

pub struct MiscPeriodic {
    relay: Relay
}

impl MiscPeriodic {
    pub fn new(relay: &Relay) -> Self {
        Self {
            relay: relay.clone()
        }
    }
}

#[async_trait]
impl IntervalFold for MiscPeriodic {
    async fn interval_fold(&mut self) -> RgResult<()> {
        self.relay.ds.count_gauges().await?;
        Ok(())
    }
}