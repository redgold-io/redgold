use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::party::address_event::AddressEvent;
use crate::party::party_events::ConfirmedExternalStakeEvent;
use crate::structs::{CurrencyAmount, PortfolioRequest, PortfolioWeighting, SupportedCurrency, Transaction, UtxoId};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PortfolioRequestEvents {
    pub events: Vec<PortfolioRequestEventInstance>,
    pub external_stake_balance_deltas: HashMap<SupportedCurrency, CurrencyAmount>,
    pub stake_utxos: Vec<(UtxoId, ConfirmedExternalStakeEvent)>,
    // Positive amount means it wants more stake, negative means it wants a withdrawal
    pub current_portfolio_imbalance: HashMap<SupportedCurrency, CurrencyAmount>,
    pub current_rdg_allocations: HashMap<SupportedCurrency, CurrencyAmount>
}


#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PortfolioRequestEventInstance {
    pub event: AddressEvent,
    pub tx: Transaction,
    pub time: i64,
    pub portfolio_request: PortfolioRequest,
    pub weightings: Vec<PortfolioWeighting>,
    pub fixed_currency_allocations: HashMap<SupportedCurrency, f64>,
    pub value_at_time: f64,
    pub portfolio_rdg_amount: CurrencyAmount,
}

impl Default for PortfolioRequestEvents {
    fn default() -> Self {
        PortfolioRequestEvents {
            events: vec![],
            external_stake_balance_deltas: Default::default(),
            stake_utxos: vec![],
            current_portfolio_imbalance: Default::default(),
            current_rdg_allocations: Default::default(),
        }
    }

}
