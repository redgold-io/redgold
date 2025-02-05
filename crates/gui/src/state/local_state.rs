use crate::components::tx_progress::PreparedTransaction;
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::structs::{AddressInfo, NetworkEnvironment, PublicKey, SupportedCurrency, Transaction};
use redgold_schema::RgResult;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum LocalStateUpdate {
    PricesPartyInfoAndDelta(PricesPartyInfoAndDeltaInitialQuery),
    HardwareSignedInternalTransaction(Transaction),
    BalanceUpdates(BalanceAddressInfoUpdate),
    // TODO: Remove this in favor of unification with other transaction handlers
    SwapResult(RgResult<PreparedTransaction>),
    RequestHardwareRefresh
}

#[derive(Clone, Debug)]
pub struct PricesPartyInfoAndDeltaInitialQuery {
    pub prices: HashMap<SupportedCurrency, f64>,
    pub party_info: HashMap<PublicKey, PartyInternalData>,
    pub delta_24hr: HashMap<SupportedCurrency, f64>,
    pub daily_one_year: HashMap<SupportedCurrency, Vec<(i64, f64)>>,
    pub on_network: NetworkEnvironment
}

#[derive(Clone, Debug)]
pub struct BalanceAddressInfoUpdate {
    pub balances: HashMap<SupportedCurrency, f64>,
    pub address_info: Option<AddressInfo>
}
