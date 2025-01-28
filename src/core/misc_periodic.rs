use crate::core::relay::Relay;
use async_trait::async_trait;
use redgold_common_no_wasm::stream_handlers::IntervalFold;
use redgold_schema::RgResult;

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
        self.relay.ds.transaction_store.delete_old_rejected_transaction(None, None).await?;
        self.relay.ds.count_gauges().await?;
        Ok(())
    }
}