use crate::helpers::easy_json::EasyJson;
use crate::party::address_event::AddressEvent;
use crate::party::external_data::{ExternalNetworkData, PriceDataPointUsdQuery};
use crate::party::party_events::{OrderFulfillment, PartyEvents};
use crate::structs::{PartyData, PublicKey, SupportedCurrency, Transaction};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::parties::{PartyState, PartyInfo, PartyMetadata, PartyInstance};


// All events associated with a 'party' composed of multiple addresses across networks.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct PartyInternalData {
    pub self_key: PublicKey,
    pub proposer_key: PublicKey,
    pub metadata: PartyMetadata,
    pub network_data: HashMap<SupportedCurrency, ExternalNetworkData>,
    pub internal_data: Vec<Transaction>,
    pub internal_address_events: Vec<AddressEvent>,
    // Technically network data / internal data above transactions are redundant in light of the
    // below field, can remove maybe later, but this is easy to use for now
    pub address_events: Vec<AddressEvent>,
    pub price_data: PriceDataPointUsdQuery,
    pub party_events: Option<PartyEvents>,
    pub locally_fulfilled_orders: Option<Vec<OrderFulfillment>>
}

impl PartyInternalData {

    pub fn clear_sensitive(&mut self) -> &mut Self {
        self
    }
    pub fn to_party_data(&self) -> PartyData {
        PartyData {
            json_party_internal_data: Some(self.json_or())
        }
    }

    pub fn self_initiated_not_debug(&self) -> bool {
        self.proposer_key == self.self_key
    }

    pub fn active_self(&self) -> bool {
        true
    }


}