use crate::structs;
use crate::structs::{Address, SupportedCurrency};
use crate::tx::external_tx::ExternalTimedTransaction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct ExternalNetworkData {
    pub address: Address,
    pub transactions: Vec<ExternalTimedTransaction>,
    // pub balance: CurrencyAmount,
    pub currency: SupportedCurrency,
    pub max_ts: Option<u64>,
    pub max_block: Option<u64>
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct UsdPrice {
    pub currency: SupportedCurrency,
    pub price: f64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct PriceDataPointUsdQuery {
    pub inner: HashMap<i64, UsdPrice>,
}
