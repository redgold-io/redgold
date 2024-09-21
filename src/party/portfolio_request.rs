use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use redgold_schema::{error_info, RgResult, SafeOption};
use redgold_schema::structs::{CurrencyAmount, PortfolioRequest, PortfolioWeighting, SupportedCurrency, Transaction, UtxoId};
use redgold_schema::util::lang_util::AnyPrinter;
use crate::party::address_event::AddressEvent;
use crate::party::party_stream::PartyEvents;
use crate::util;
use chrono::{Utc, TimeZone, Duration, Datelike};
use itertools::Itertools;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use redgold_common::external_resources::ExternalNetworkResources;
use crate::party::order_fulfillment::OrderFulfillment;
use crate::party::stake_event_stream::ConfirmedExternalStakeEvent;

#[derive(Clone, Serialize, Deserialize)]
pub struct PortfolioRequestEvents {
    pub events: Vec<PortfolioRequestEventInstance>,
    pub external_stake_balance_deltas: HashMap<SupportedCurrency, CurrencyAmount>,
    pub stake_utxos: Vec<(UtxoId, ConfirmedExternalStakeEvent)>,
    pub current_portfolio_imbalance: HashMap<SupportedCurrency, CurrencyAmount>
}


#[derive(Clone, Serialize, Deserialize)]
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
        }
    }

}

impl PartyEvents {


    pub async fn calculate_update_portfolio_imbalance(&mut self) -> RgResult<()> {
        let imbalance = self.calculate_portfolio_imbalance().await?;
        self.portfolio_request_events.current_portfolio_imbalance = imbalance;
        Ok(())
    }
    // Represents amount missing from requested fulfillment, negative means excess
    pub async fn calculate_portfolio_imbalance(&self) -> RgResult<HashMap<SupportedCurrency, CurrencyAmount>> {

        let usd_rdg = self.usd_rdg_estimate().unwrap_or(100.0);
        let mut requested_allocations = HashMap::new();
        requested_allocations.insert(SupportedCurrency::Bitcoin, CurrencyAmount::zero(SupportedCurrency::Bitcoin));
        requested_allocations.insert(SupportedCurrency::Ethereum, CurrencyAmount::zero(SupportedCurrency::Ethereum));
        let relay = self.relay.as_ref().unwrap();
        for e in self.portfolio_request_events.events.iter() {
            let rdg_amount = e.portfolio_rdg_amount.clone();
            for (cur, alloc) in e.fixed_currency_allocations.clone() {
                let p = relay.ds.price_time.max_time_price_by(cur.clone(), e.time.clone()).await?;
                let usd_value = rdg_amount.to_fractional() * alloc * usd_rdg;
                if let Some(usd_p_pair) = p {
                    let pair_amount = usd_value / usd_p_pair;
                    if let Ok(amt) = CurrencyAmount::from_fractional_cur(pair_amount, cur.clone()) {
                        let current_amount = requested_allocations.get(&cur).unwrap().clone();
                        requested_allocations.insert(cur.clone(), current_amount + amt);
                    }
                }
            }
        }

        let mut delta_allocation = HashMap::new();
        delta_allocation.insert(SupportedCurrency::Bitcoin, CurrencyAmount::zero(SupportedCurrency::Bitcoin));
        delta_allocation.insert(SupportedCurrency::Ethereum, CurrencyAmount::zero(SupportedCurrency::Ethereum));

        for (k, v) in self.portfolio_request_events.external_stake_balance_deltas.iter() {
            let requested = requested_allocations.get(k).unwrap().clone();
            let fulfilled = v.clone();
            let delta = requested - fulfilled;
            delta_allocation.insert(k.clone(), delta);
        }
        Ok(delta_allocation)
    }

    // pub fn get_value_usd(&self, redgold_amount: &CurrencyAmount, fraction: f64, currency: SupportedCurrency) -> f64 {
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

    pub fn usd_rdg_estimate(&self) -> RgResult<f64> {
        self.central_prices.iter().map(|(k, v)| v.min_bid_estimated).reduce(f64::max).ok_msg("Missing price data")
    }

    pub fn handle_maybe_portfolio_stake_event(&mut self, ev: ConfirmedExternalStakeEvent) -> RgResult<()> {
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

    pub fn handle_maybe_portfolio_stake_withdrawal_event(&mut self, f: (OrderFulfillment, AddressEvent, AddressEvent), t: ExternalTimedTransaction) {
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

    pub fn handle_portfolio_request(&mut self, event: &AddressEvent, time: i64, tx: &Transaction) -> RgResult<()> {
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

pub fn get_most_recent_day_millis() -> i64 {
    let now = Utc::now();
    let today_start = Utc.with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0);
    let timestamp = today_start.unwrap().timestamp_millis();
    timestamp
}

pub fn get_most_recent_day_millis_from_millis(time: i64) -> i64 {
    let now = Utc.timestamp_millis_opt(time).unwrap();
    let today_start = Utc.with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0);
    let timestamp = today_start.unwrap().timestamp_millis();
    timestamp
}

#[test]
fn recent() {
    get_most_recent_day_millis().print();
}