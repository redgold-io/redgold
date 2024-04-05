use async_trait::async_trait;
use redgold_schema::{ErrorInfoContext, RgResult, SafeOption};
use redgold_schema::observability::errors::EnhanceErrorInfo;
use crate::core::relay::Relay;
use crate::core::stream_handlers::IntervalFold;
use crate::util::cmd::{run_bash, run_cmd};

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

pub fn disk_space_gb() -> RgResult<u32> {
    let (stdout, stderr) = run_bash(r#"df -Ph . | awk 'NR==2 {print $4}'"#)?;
    let gigs = stdout.split("G").next();
    let gb = gigs.safe_get_msg("Err in getting disk space").add(stderr.clone())?;
    gb.parse::<u32>().error_info("Err in parsing disk space").add(stderr.clone())
}
#[test]
pub fn disk_space_test() {
    let gb = disk_space_gb().expect("");
    println!("Gigs disk {gb}")
}