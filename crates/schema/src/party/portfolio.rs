use crate::helpers::with_metadata_hashable::WithMetadataHashable;
use crate::party::address_event::AddressEvent;
use crate::party::party_events::ConfirmedExternalStakeEvent;
use crate::structs::{CurrencyAmount, Hash, PortfolioRequest, PortfolioWeighting, SupportedCurrency, Transaction, UtxoId};
use itertools::Either::{Left, Right};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PortfolioRequestEvents {
    pub events: Vec<PortfolioRequestEventInstance>,
    pub external_stake_balance_deltas: HashMap<SupportedCurrency, CurrencyAmount>,
    pub stake_utxos: Vec<(UtxoId, ConfirmedExternalStakeEvent)>,
    // Positive amount means it wants more stake, negative means it wants a withdrawal
    pub current_portfolio_imbalance: HashMap<SupportedCurrency, CurrencyAmount>,
    pub current_rdg_allocations: HashMap<SupportedCurrency, CurrencyAmount>,
    // fulfilled, unfulfilled usd value.
    pub enriched_events: Option<HashMap<Hash, HashMap<SupportedCurrency, (f64, f64)>>>
}

impl PortfolioRequestEvents {
    pub fn calculate_current_fulfillment_by_event(&self)
        -> HashMap<Hash, HashMap<SupportedCurrency, (f64, f64)>> {
        let mut all_events = vec![];
        for e in self.events.iter() {
            all_events.push((e.time, Left(e.clone())));
        }
        for e in self.stake_utxos.iter() {
            let time = (e.1.ett.timestamp.unwrap() as i64, Right(e.1.clone()));
            all_events.push(time);
        }
        all_events.sort_by(|a, b| a.0.cmp(&b.0));


        // Need to keep track here of the current remaining unfulfilled amounts, in order
        // of request events processed,
        let mut cur_fulfills: HashMap<SupportedCurrency, Vec<(Hash, (f64, f64))>> = HashMap::new();

        // This function is in USD, but its only for approximating values for display purposes.
        // Values are actually settled in RDG at current time of.

        let mut tx_hash_to_fulfilled_amounts = HashMap::new();

        for (_, e) in all_events.iter() {
            match e {
                Left(e) => {
                    let starting_usd = e.value_at_time;
                    let value_usd_by_cur = e.fixed_currency_allocations.iter()
                        .map(|(k,v)| {
                            (k.clone(), v * starting_usd)
                        }).collect::<HashMap<SupportedCurrency, f64>>();

                    for (c, u) in value_usd_by_cur.into_iter() {
                        let x = cur_fulfills.entry(c.clone()).or_insert(vec![]);
                        x.push((e.tx.hash_or(), (0.0,u.clone())));
                    }
                    // this is a request
                },
                Right(e) => {
                    let amt = e.ett.currency_amount();
                    if let Some(p_usd) = e.ett.price_usd {
                        let mut usd_value = amt.to_fractional() * p_usd;
                        let cur = e.ett.currency;
                        // this is someone supplying the stake for that request.
                        if let Some(vec) = cur_fulfills.get_mut(&cur) {
                            // Iterate over the vec, remove an element if it is fully fulfilled.
                            vec.retain_mut(|(_hash, (cur_fulfilled, remaining))| {
                                if usd_value == 0.0 {
                                    true
                                } else {
                                    if remaining.clone() > usd_value {
                                        *remaining -= usd_value;
                                        *cur_fulfilled += usd_value;
                                        usd_value = 0.0;
                                        true
                                    } else {
                                        *cur_fulfilled += remaining.clone();
                                        *remaining = 0.0;
                                        usd_value -= remaining.clone();
                                        false
                                    }
                                }
                            });

                        }

                    }
                }
            }
        }
        // For all the remaining unfulfilled amounts, move them into the final
        // returned value
        for (c, v) in cur_fulfills.iter() {
            for (hash, (fulfilled, remaining)) in v.iter() {
                let entry = tx_hash_to_fulfilled_amounts.entry(hash.clone()).or_insert(HashMap::new());
                entry.insert(c.clone(), (fulfilled.clone(), remaining.clone()));
            }
        }

        tx_hash_to_fulfilled_amounts
    }
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
            enriched_events: None,
        }
    }

}
