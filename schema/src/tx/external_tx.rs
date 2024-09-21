use serde::{Deserialize, Serialize};
use crate::{structs, RgResult};
use crate::structs::{CurrencyAmount, SupportedCurrency};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct ExternalTimedTransaction {
    pub tx_id: String,
    pub timestamp: Option<u64>,
    pub other_address: String,
    pub other_output_addresses: Vec<String>,
    pub amount: u64,
    pub bigint_amount: Option<String>,
    pub incoming: bool,
    pub currency: SupportedCurrency,
    pub block_number: Option<u64>,
    pub price_usd: Option<f64>,
    pub fee: Option<CurrencyAmount>,
}

impl ExternalTimedTransaction {

    pub fn balance_change(&self) -> CurrencyAmount {
        let fee = self.fee.clone().unwrap_or(CurrencyAmount::zero(self.currency));
        if self.incoming {
            self.currency_amount()
        } else {
            self.currency_amount() - fee
        }
    }

    pub fn currency_amount(&self) -> CurrencyAmount {
        let mut ca = if let Some(ba) = self.bigint_amount.as_ref() {
            CurrencyAmount::from_eth_bigint_string(ba.clone())
        } else {
            CurrencyAmount::from(self.amount as i64)
        };
        ca.currency = Some(self.currency as i32);
        ca
    }
    pub fn confirmed(&self) -> bool {
        self.timestamp.is_some()
    }


}