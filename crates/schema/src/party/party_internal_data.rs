use crate::helpers::easy_json::EasyJson;
use crate::party::address_event::AddressEvent;
use crate::party::external_data::{ExternalNetworkData, PriceDataPointUsdQuery};
use crate::party::party_events::{OrderFulfillment, PartyEvents};
use crate::structs::{PartyData, PartyInfo, SupportedCurrency, Transaction};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PartyInternalData {
    pub party_info: PartyInfo,
    pub network_data: HashMap<SupportedCurrency, ExternalNetworkData>,
    pub internal_data: Vec<Transaction>,
    // Technically network data / internal data above transactions are redundant in light of the
    // below field, can remove maybe later, but this is easy to use for now
    pub address_events: Vec<AddressEvent>,
    pub price_data: PriceDataPointUsdQuery,
    pub party_events: Option<PartyEvents>,
    pub locally_fulfilled_orders: Option<Vec<OrderFulfillment>>
}

impl PartyInternalData {

    pub fn clear_sensitive(&mut self) -> &mut Self {
        self.party_info.clear_sensitive();
        self
    }
    pub fn to_party_data(&self) -> PartyData {
        PartyData {
            json_party_internal_data: Some(self.json_or())
        }
    }

    pub fn not_debug(&self) -> bool {
        self.party_info.not_debug()
    }

    pub fn self_initiated_not_debug(&self) -> bool {
        self.party_info.not_debug() && self.party_info.self_initiated.unwrap_or(false)
    }

    pub fn active_self(&self) -> bool {
        self.party_info.active() && self.party_info.self_initiated.unwrap_or(false)
    }


}