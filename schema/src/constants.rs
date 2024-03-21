

// 44 for non-segwit to match Ethereum equivalent since witness data is included in transactions
// 84 would be if we were excluding the witness info like Segwit
pub const REDGOLD_PURPOSE: i64 = 44;
pub const REDGOLD_KEY_DERIVATION_PATH: i64 = 16180;
pub const DECIMAL_MULTIPLIER: i64 = 1e8 as i64;
pub const DECIMALS: i64 = 8;
pub const MAX_COIN_SUPPLY: i64 = 1e7 as i64;
pub const REWARD_CYCLES: i64 = 100;
pub const REWARD_AMOUNT: i64 = 1e5 as i64;
pub const REWARD_AMOUNT_RAW: i64 = REWARD_AMOUNT * DECIMAL_MULTIPLIER;


pub const MAX_INPUTS_OUTPUTS: u32 = 5000;

pub const STANDARD_FINALIZATION_INTERVAL_MILLIS: u64 = 4_000;
pub const DEBUG_FINALIZATION_INTERVAL_MILLIS: u64 = 4000;
pub const OBSERVATION_FORMATION_TIME_MILLIS: u64 = 3_000;
pub const REWARD_POLL_INTERVAL: u64 = 60_000;

pub const EARLIEST_TIME: i64 = 1650866635183;
//
// pub const HASHER: Code = multihash::Code::Sha3_256;
// pub const ADDRESS_HASHER: Code = multihash::Code::Sha3_224;

pub const VERSION: i32 = 0;

pub const REDGOLD: &str = "redgold";

pub const RDG_SYMBOL: &str = "RDG";

pub const STANDARD_VERSION: i64 = 0;

pub fn default_node_internal_derivation_path(account: i64) -> String {
    format!("m/{REDGOLD_PURPOSE}'/{REDGOLD_KEY_DERIVATION_PATH}'/{account}'/0/0")
}

pub fn redgold_keypair_change_path(change: i64) -> String {
    format!("m/{REDGOLD_PURPOSE}'/{REDGOLD_KEY_DERIVATION_PATH}'/0'/0/{change}")
}
