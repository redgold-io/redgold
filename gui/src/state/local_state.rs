use std::collections::HashMap;
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::RgResult;
use redgold_schema::structs::{AddressInfo, PublicKey, SupportedCurrency, Transaction};
use crate::components::tx_progress::PreparedTransaction;

#[derive(Clone, Debug)]
pub enum LocalStateUpdate {
    PricesPartyInfoAndDelta(PricesPartyInfoAndDelta),
    HardwareSignedInternalTransaction(Transaction),
    BalanceUpdates(BalanceAddressInfoUpdate),
    // TODO: Remove this in favor of unification with other transaction handlers
    SwapResult(RgResult<PreparedTransaction>),
    RequestHardwareRefresh
}

#[derive(Clone, Debug)]
pub struct PricesPartyInfoAndDelta {
    pub prices: HashMap<SupportedCurrency, f64>,
    pub party_info: HashMap<PublicKey, PartyInternalData>,
    pub delta_24hr: HashMap<SupportedCurrency, f64>,
}

#[derive(Clone, Debug)]
pub struct BalanceAddressInfoUpdate {
    pub balances: HashMap<SupportedCurrency, f64>,
    pub address_info: Option<AddressInfo>
}
