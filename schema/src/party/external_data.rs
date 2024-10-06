use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::structs;
use crate::structs::SupportedCurrency;
use crate::tx::external_tx::ExternalTimedTransaction;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ExternalNetworkData {
    pub pk: structs::PublicKey,
    pub transactions: Vec<ExternalTimedTransaction>,
    // pub balance: CurrencyAmount,
    pub currency: SupportedCurrency,
    pub max_ts: Option<u64>,
    pub max_block: Option<u64>
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct UsdPrice {
    pub currency: SupportedCurrency,
    pub price: f64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct PriceDataPointUsdQuery {
    pub inner: HashMap<i64, UsdPrice>,
}
