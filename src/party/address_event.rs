use redgold_schema::structs::{PublicKey, State, SupportedCurrency, ValidationLiveness};
use rocket::serde::{Deserialize, Serialize};
use itertools::Itertools;
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use crate::party::party_stream::TransactionWithObservationsAndPrice;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum AddressEvent {
    External(ExternalTimedTransaction),
    Internal(TransactionWithObservationsAndPrice)
}


impl AddressEvent {

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
