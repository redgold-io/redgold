use crate::util;
use chrono::{Datelike, Duration, TimeZone, Utc};
use itertools::Itertools;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_data::data_store::DataStore;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::party::address_event::AddressEvent;
use redgold_schema::party::party_events::ConfirmedExternalStakeEvent;
use redgold_schema::party::party_events::OrderFulfillment;
use redgold_schema::party::party_events::PartyEvents;
use redgold_schema::party::portfolio::PortfolioRequestEventInstance;
use redgold_schema::structs::{CurrencyAmount, PortfolioRequest, PortfolioWeighting, SupportedCurrency, Transaction, UtxoId};
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use redgold_schema::util::lang_util::AnyPrinter;
use redgold_schema::{error_info, RgResult, SafeOption};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub trait PortfolioEventMethods {
    async fn calculate_update_portfolio_imbalance<T>(&mut self, ds: &T) -> RgResult<()> where T: ExternalNetworkResources + Send;
    async fn calculate_portfolio_imbalance<T>(&mut self, ds: &T) -> RgResult<HashMap<SupportedCurrency, CurrencyAmount>> where T: ExternalNetworkResources + Send;
    fn usd_rdg_estimate(&self) -> RgResult<f64>;
    fn handle_maybe_portfolio_stake_event(&mut self, ev: ConfirmedExternalStakeEvent) -> RgResult<()>;
    fn handle_maybe_portfolio_stake_withdrawal_event(&mut self, f: (OrderFulfillment, AddressEvent, AddressEvent), t: ExternalTimedTransaction);
    fn handle_portfolio_request(&mut self, event: &AddressEvent, time: i64, tx: &Transaction) -> RgResult<()>;
}

impl PortfolioEventMethods for PartyEvents {
    
    async fn calculate_update_portfolio_imbalance<T>(&mut self, ds: &T) -> RgResult<()> where T: ExternalNetworkResources + Send {
        let imbalance = self.calculate_portfolio_imbalance(ds).await?;
        self.portfolio_request_events.current_portfolio_imbalance = imbalance;
        Ok(())
    }
    // Represents amount missing from requested fulfillment, negative means excess
    async fn calculate_portfolio_imbalance<T>(&mut self, ds: &T) -> RgResult<HashMap<SupportedCurrency, CurrencyAmount>> where T: ExternalNetworkResources + Send {

        let usd_rdg = self.usd_rdg_estimate().unwrap_or(100.0);
        let mut requested_allocations = HashMap::new();
        requested_allocations.insert(SupportedCurrency::Bitcoin, CurrencyAmount::zero(SupportedCurrency::Bitcoin));
        requested_allocations.insert(SupportedCurrency::Ethereum, CurrencyAmount::zero(SupportedCurrency::Ethereum));
        let mut rdg_allocations: HashMap<SupportedCurrency, CurrencyAmount> = HashMap::new();

        for e in self.portfolio_request_events.events.iter() {
            let rdg_amount = e.portfolio_rdg_amount.clone();
            for (cur, alloc) in e.fixed_currency_allocations.clone() {
                let p = ds.max_time_price_by(cur.clone(), e.time.clone()).await?;
                let rdg_alloc = rdg_amount.to_fractional() * alloc;
                let rdg_alloc_c = CurrencyAmount::from_fractional(rdg_alloc).unwrap();
                if let Some(amt) = rdg_allocations.get(&cur) {
                    let mut sum = amt.clone();
                    sum = sum + rdg_alloc_c.clone();
                    rdg_allocations.insert(cur.clone(), sum);
                } else {
                    rdg_allocations.insert(cur.clone(), rdg_alloc_c);
                }
                let usd_value = rdg_alloc * usd_rdg;
                if let Some(usd_p_pair) = p {
                    let pair_amount = usd_value / usd_p_pair;
                    if let Ok(amt) = CurrencyAmount::from_fractional_cur(pair_amount, cur.clone()) {
                        let current_amount = requested_allocations.get(&cur).unwrap().clone();
                        requested_allocations.insert(cur.clone(), current_amount + amt);
                    }
                } else {
                    tracing::error!("Missing price data for {}", cur.json_or());
                }
            }
        }
        self.portfolio_request_events.current_rdg_allocations = rdg_allocations.clone();

        let mut delta_allocation = HashMap::new();
        delta_allocation.insert(SupportedCurrency::Bitcoin, CurrencyAmount::zero(SupportedCurrency::Bitcoin));
        delta_allocation.insert(SupportedCurrency::Ethereum, CurrencyAmount::zero(SupportedCurrency::Ethereum));

        for (k, requested) in requested_allocations.iter() {
            let v = self.portfolio_request_events.external_stake_balance_deltas.get(k);
            let delta = if let Some(v) = v {
                requested.clone() - v.clone()
            } else {
                requested.clone()
            };
            delta_allocation.insert(k.clone(), delta);
        }
        Ok(delta_allocation)
    }

    // fn get_value_usd(&self, redgold_amount: &CurrencyAmount, fraction: f64, currency: SupportedCurrency) -> f64 {
    //     let relative_amount = (redgold_amount.amount as f64) * fraction;
    //     let price = self.central_prices.get(&currency).map(
    //         |p| {
    //             let pair_amount = relative_amount / p.min_bid;  //  RDG / (RDG/PAIR) = PAIR
    //             pair_amount * p.pair_quote_price_estimate
    //         }
    //
    //     );
    //     let usd_value = price * redgold_amount.to_float();
    //     usd_value * fraction
    // }

    fn usd_rdg_estimate(&self) -> RgResult<f64> {
        self.central_prices.iter().map(|(k, v)| v.min_bid_estimated).reduce(f64::max).ok_msg("Missing price data")
    }

    fn handle_maybe_portfolio_stake_event(&mut self, ev: ConfirmedExternalStakeEvent) -> RgResult<()> {
        if let Some(d) = ev.pending_event.tx.stake_deposit_request() {
            if d.portfolio_fulfillment_params.is_some() {
                let ca = ev.ett.currency_amount();
                if let Some(b) = self.portfolio_request_events.external_stake_balance_deltas.get_mut(&ca.currency_or()) {
                    *b = b.clone() + ev.ett.currency_amount();
                } else {
                    self.portfolio_request_events.external_stake_balance_deltas.insert(ca.currency_or(), ca);
                }
                let transaction = ev.pending_event.tx.clone();
                let vec = transaction.stake_requests();
                let (utxo_id, e) = vec.get(0).unwrap();
                self.portfolio_request_events.stake_utxos.push((utxo_id.clone(), ev.clone()));
            }
        }
        Ok(())
    }

    fn handle_maybe_portfolio_stake_withdrawal_event(&mut self, f: (OrderFulfillment, AddressEvent, AddressEvent), t: ExternalTimedTransaction) {
        let (of, init, end) = f;
        match init {
            AddressEvent::Internal(e) => {
                let utxo_id_match = e.tx.input_utxo_ids().filter_map(|u|
                    self.portfolio_request_events.stake_utxos.iter().filter(|(u2, ev)| u == u2).next()
                ).next();
                if let Some((utxo_id, ev)) = utxo_id_match {
                    let ca = ev.ett.currency_amount();
                    if let Some(b) = self.portfolio_request_events.external_stake_balance_deltas.get_mut(&ca.currency_or()) {
                        *b = b.clone() - ca;
                    }
                    self.portfolio_request_events.stake_utxos = self.portfolio_request_events.stake_utxos.iter().filter(|(u, _)| u != utxo_id).map(|x| x.clone()).collect();
                }
            }
            _ => {
            }
        }
    }

    fn handle_portfolio_request(&mut self, event: &AddressEvent, time: i64, tx: &Transaction) -> RgResult<()> {
        let portfolio_rdg_amount = tx.outputs_of_pk(&self.party_public_key)?.filter_map(|o| o.opt_amount())
            .sum::<i64>();
        let portfolio_rdg_amount = CurrencyAmount::from(portfolio_rdg_amount);
        let prices = match event {
            AddressEvent::External(_) => { Err(error_info("External address event not supported"))? }
            AddressEvent::Internal(e) => {
                e.all_relevant_prices_usd.clone()
            }
        };
        if let Some(pr) = tx.portfolio_request() {
            if let Some(pi) = pr.portfolio_info.as_ref() {
                let weights = pi.portfolio_weightings.clone();
                let alloc = pi.fixed_currency_allocations();
                let value_at_time = self.usd_rdg_estimate().unwrap_or(0.0) * portfolio_rdg_amount.to_fractional();
                self.portfolio_request_events.events.push(PortfolioRequestEventInstance {
                    event: event.clone(),
                    tx: tx.clone(),
                    time,
                    portfolio_request: pr.clone(),
                    weightings: weights,
                    fixed_currency_allocations: alloc,
                    portfolio_rdg_amount,
                    value_at_time,
                })
            }
        }
        Ok(())
    }
}

pub(crate) fn get_most_recent_day_millis() -> i64 {
    let now = Utc::now();
    let today_start = Utc.with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0);
    let timestamp = today_start.unwrap().timestamp_millis();
    timestamp
}

fn get_most_recent_day_millis_from_millis(time: i64) -> i64 {
    let now = Utc.timestamp_millis_opt(time).unwrap();
    let today_start = Utc.with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0);
    let timestamp = today_start.unwrap().timestamp_millis();
    timestamp
}

#[test]
fn recent() {
    get_most_recent_day_millis().print();
}