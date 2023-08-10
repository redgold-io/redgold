use redgold_schema::RgResult;
use crate::core::relay::Relay;
use crate::core::stream_handlers::IntervalFold;

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

impl IntervalFold for Shuffle {
    async fn interval_fold(&mut self) -> RgResult<()> {
        // TODO: Calculate used disk space
        // Separate fast access from slow access disk space if needed.
        // For now treat identically
        Ok(())
    }
}