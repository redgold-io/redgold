
use multihash::Code;

pub const REDGOLD_KEY_DERIVATION_PATH: i64 = 1_854_715_124;
pub const DECIMAL_MULTIPLIER: i64 = 1e8 as i64;
pub const DECIMALS: i64 = 8;
pub const MAX_COIN_SUPPLY: i64 = 1e8 as i64;
pub const REWARD_CYCLES: i64 = 1000;
pub const REWARD_AMOUNT: i64 = 1e5 as i64;
pub const REWARD_AMOUNT_RAW: i64 = REWARD_AMOUNT * DECIMAL_MULTIPLIER;

#[test]
fn test_max() {
    assert_eq!(PRIME_MULTIPLIER, PRIME_DIVISORS);
    assert_eq!(REWARD_CYCLES, REWARD_DIVISORS);
    assert_eq!(REWARD_AMOUNT, MAX_COIN_SUPPLY / REWARD_CYCLES);
}

pub const MAX_INPUTS_OUTPUTS: u32 = 5000;
pub const MIN_FEE_RAW: i64 = DECIMAL_MULTIPLIER / 100;

pub const DEFAULT_PORT_OFFSET: u16 = 16180;
#[allow(dead_code)]
pub const TESTNET_PORT_OFFSET: u16 = 16280;

pub const STANDARD_FINALIZATION_INTERVAL_MILLIS: u64 = 4_000;
pub const DEBUG_FINALIZATION_INTERVAL_MILLIS: u64 = 4000;
pub const OBSERVATION_FORMATION_TIME_MILLIS: u64 = 3_000;
pub const REWARD_POLL_INTERVAL: u64 = 60_000;

pub const EARLIEST_TIME: u64 = 1650866635183;

pub const HASHER: Code = multihash::Code::Sha3_512;

pub const VERSION: i32 = 0;

pub const REDGOLD: &str = "redgold";

pub const SYMBOL: &str = "RDG";

pub const STANDARD_VERSION: i64 = 0;
