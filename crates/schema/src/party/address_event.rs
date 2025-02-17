use crate::helpers::easy_json::EasyJson;
use crate::helpers::with_metadata_hashable::WithMetadataHashable;
use crate::structs::{Address, ObservationProof, PublicKey, State, SupportedCurrency, Transaction, ValidationLiveness};
use crate::tx::external_tx::ExternalTimedTransaction;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum AddressEvent {
    External(ExternalTimedTransaction),
    Internal(TransactionWithObservationsAndPrice)
}

// is this the right model?
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum AddressEventType {
    SwapRequest,
    ImplicitSwapRequest,
    StakeInternalDeposit,
    StakeExternalRequest,
    StakeExternalFill,
    SwapFulfillment,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct EnrichedAddressEvent {
    event: AddressEvent,

}


impl AddressEvent {

    pub fn incoming(&self) -> bool {
        match self {
            AddressEvent::External(e) => e.incoming,
            AddressEvent::Internal(t) => !t.tx
                .input_address_descriptor_address_or_public_key().contains(&t.queried_address)
        }
    }

    pub fn internal_external_str(&self) -> String {
        match self {
            AddressEvent::External(_) => "external".to_string(),
            AddressEvent::Internal(_) => "internal".to_string()
        }
    }

    pub fn other_swap_address(&self) -> Option<String> {
        match self {
            AddressEvent::External(e) => Some(e.other_address.clone()),
            AddressEvent::Internal(t) =>
            if t.tx.is_swap() {
                t.tx.first_input_address()
                    .and_then(|a| a.render_string().ok())
            } else {
                None
            }
        }
    }

    pub fn identifier(&self) -> String {
        match self {
            AddressEvent::External(e) => e.tx_id.clone(),
            AddressEvent::Internal(t) => t.tx.hash_or().hex()
        }
    }

    pub fn ser_tx(&self) -> String {
        match self {
            AddressEvent::External(e) => e.json_or(),
            AddressEvent::Internal(t) => t.tx.json_or()
        }
    }

    pub fn external_currency(&self) -> Option<SupportedCurrency> {
        match self {
            AddressEvent::External(e) => Some(e.currency),
            AddressEvent::Internal(t) => t.tx.external_destination_currency().clone()
        }
    }

    pub fn currency(&self) -> SupportedCurrency {
        match self {
            AddressEvent::External(e) => e.currency,
            AddressEvent::Internal(_) => SupportedCurrency::Redgold
        }
    }

    pub fn usd_event_price(&self) -> Option<f64> {
        {match self {
            AddressEvent::External(e) => {e.price_usd}
            AddressEvent::Internal(e) => {e.price_usd}
        }}.clone()
    }
    // pub fn other_addresses(&self) -> HashSet<Address> {
    //     match self {
    //         AddressEvent::External(e) => e.tx_id.clone(),
    //         AddressEvent::Internal(t) => t.tx.hash_or().hex()
    //     }
    // }

    pub fn time(&self, seeds: &Vec<PublicKey>) -> Option<i64> {
        match self {
            // Convert from unix time to time ms
            AddressEvent::External(e) => e.timestamp.map(|t| t as i64),
            AddressEvent::Internal(t) => {
                let seed_obs = t.observations.iter().filter_map(|o|
                    {
                        let metadata = o.proof.as_ref()
                            .and_then(|p| p.public_key.as_ref())
                            .filter(|pk| seeds.contains(pk))
                            .and_then(|_pk| o.metadata.as_ref());
                        metadata
                            .and_then(|m| m.time().ok())
                            .filter(|_| metadata.filter(|m| m.validation_liveness == ValidationLiveness::Live as i32).is_some())
                            .filter(|_| metadata.filter(|m| m.state == State::Accepted as i32).is_some())
                    }
                ).map(|t| t.clone()).collect_vec();
                if seeds.len() == 0 {
                    t.tx.time().cloned().ok()
                } else {
                    let times = seed_obs.iter().sum::<i64>();
                    let avg = times / seed_obs.len() as i64;
                    if avg == 0 {
                        None
                    } else {
                        Some(avg)
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct TransactionWithObservationsAndPrice {
    pub tx: Transaction,
    pub observations: Vec<ObservationProof>,
    pub price_usd: Option<f64>,
    pub all_relevant_prices_usd: HashMap<SupportedCurrency, f64>,
    pub queried_address: Address
}
