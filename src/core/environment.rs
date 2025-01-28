use std::time::Duration;

/*

pub const STANDARD_FINALIZATION_INTERVAL_MILLIS: u64 = 60_000;
pub const DEBUG_FINALIZATION_INTERVAL_MILLIS: u64 = 2000;
pub const OBSERVATION_FORMATION_TIME_MILLIS: u64 = 5_000;
pub const REWARD_POLL_INTERVAL: u64 = 60_000;

 */
struct NetworkDefaults {
    port_offset: i16,
    finalization_minimum_time: Duration,
    observation_formation_time: Duration,
}
