use async_trait::async_trait;
use redgold_schema::RgResult;
use crate::core::relay::Relay;
use redgold_common_no_wasm::stream_handlers::IntervalFold;

pub struct Shuffle {
    relay: Relay
}

impl Shuffle {
    pub fn new(relay: &Relay) -> Self {
        Self {
            relay: relay.clone()
        }
    }
}

#[async_trait]
impl IntervalFold for Shuffle {

    async fn interval_fold(&mut self) -> RgResult<()> {

        // df -Ph . | tail -1 | awk '{print $4}'
        // TODO: Calculate used disk space
        // Separate fast access from slow access disk space if needed.
        // For now treat identically
        Ok(())
    }
}