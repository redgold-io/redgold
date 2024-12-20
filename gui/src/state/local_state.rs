use std::collections::HashMap;
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::structs::{PublicKey, SupportedCurrency};

#[derive(Clone, Debug)]
pub enum LocalStateUpdate {
    PricesPartyInfoAndDelta(PricesPartyInfoAndDelta),
}

#[derive(Clone, Debug)]
pub struct PricesPartyInfoAndDelta {
    pub prices: HashMap<SupportedCurrency, f64>,
    pub party_info: HashMap<PublicKey, PartyInternalData>,
    pub delta_24hr: HashMap<SupportedCurrency, f64>,
}
