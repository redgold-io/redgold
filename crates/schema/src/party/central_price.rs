use crate::helpers::easy_json::EasyJson;
use crate::party::address_event::AddressEvent;
use crate::party::party_events::{OrderFulfillment, PartyEvents};
use crate::party::price_volume::PriceVolume;
use crate::structs::{Address, CurrencyAmount, ExternalTransactionId, NetworkEnvironment, SupportedCurrency};
use crate::tx::external_tx::ExternalTimedTransaction;
use crate::{RgResult, SafeOption};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const DUST_LIMIT: i64 = 2500;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct CentralPricePair {
    // Denominated in RDG/BTC for example
    pub min_ask: f64,
    // Denominated in USD/RDG for example
    pub min_ask_estimated: f64,
    // Denominated in RDG/BTC for example
    pub min_bid: f64,
    // Denominated in USD / RDG for example, how many USD equivalent for BTC will we give for X RDG
    pub min_bid_estimated: f64,
    // Oracle resolved event time for closest trade event
    pub time: i64,
    // RDG
    pub base_currency: SupportedCurrency,
    // BTC, ETH, etc.
    pub pair_quote_currency: SupportedCurrency,
    // USD typically
    pub pricing_estimate_pair: SupportedCurrency,
    // RDG / BTC example
    pub reserve_ratio_pair: f64,
    pub base_volume: CurrencyAmount,
    pub pair_quote_volume: CurrencyAmount,
    // USD / PAIR
    pub pair_quote_price_estimate: f64,
}

impl CentralPricePair {

    pub fn default_divisions() -> i32 {
        40
    }
    pub fn default_scale() -> f64 {
        20.0f64
    }

    pub fn default_reserve_fraction() -> f64 {
        0.1
    }
    pub fn bids(&self) -> Vec<PriceVolume> {
        let vol = self.pair_quote_volume.amount_i64_or() as f64;
        let vol = (vol * (1.0f64 - Self::default_reserve_fraction())) as u64;
        PriceVolume::generate(
            vol,
            self.min_bid, // Price here is RDG/BTC
                Self::default_divisions(),
            self.min_bid*0.9,
            Self::default_scale() / 2.0
        )
    }

    pub fn bids_usd(&self) -> Vec<PriceVolume> {
        self.bids().into_iter().map(|pv| {
            PriceVolume {
                // price is RDG / BTC, price estimate is USD / BTC
                price: self.pair_quote_price_estimate / pv.price,
                volume: pv.volume
            }
        }).collect()
    }
    pub fn asks(&self) -> Vec<PriceVolume> {
        let vol = self.base_volume.amount_i64_or() as f64;
        let vol = (vol * (1.0 - Self::default_reserve_fraction())) as u64;
        PriceVolume::generate(
            vol,
            self.min_ask, // Price here is RDG/BTC
                Self::default_divisions(),
            -0.5*self.min_ask,
            self.min_ask * 1.0
        )
    }

    pub fn asks_usd(&self) -> Vec<PriceVolume> {
        self.asks().into_iter().map(|pv| {
            PriceVolume {
                // price is RDG / BTC, price estimate is USD / BTC
                price: self.pair_quote_price_estimate / pv.price,
                volume: pv.volume
            }
        }).collect()
    }

    pub fn dummy_fulfill(
        &self,
        order_amount_typed: CurrencyAmount,
        order_amount: u64,
        is_ask: bool,
        network: &NetworkEnvironment,
        currency: SupportedCurrency
    ) -> f64 {
        let mut address = Address::default();
        address.currency = currency as i32;
        self.fulfill_taker_order(
            order_amount_typed,
            order_amount,
            is_ask,
            0,
            None,
            &address,
            AddressEvent::External(ExternalTimedTransaction::default()),
            network
        ).map(|f| f.fulfilled_currency_amount().to_fractional()).unwrap_or(0.0)
    }

    pub fn fulfill_taker_order(
        &self,
        order_amount_typed: CurrencyAmount,
        order_amount: u64,
        is_ask: bool,
        event_time: i64,
        tx_id: Option<ExternalTransactionId>,
        destination: &Address,
        primary_event: AddressEvent,
        network: &NetworkEnvironment
    ) -> Option<OrderFulfillment> {
        let from_rdg = order_amount_typed.currency_or() == SupportedCurrency::Redgold;
        let fulfilled_amount_usd = if from_rdg {
            order_amount_typed.to_fractional() * 100.0f64
        } else {
            order_amount_typed.to_fractional() * self.pair_quote_price_estimate * 0.98f64
        };

        let fulfilled_amt_cur = if from_rdg {
            fulfilled_amount_usd / self.pair_quote_price_estimate
        } else {
            fulfilled_amount_usd / 100.0f64
        };

        let fulfilled_cur = if from_rdg {
            destination.currency_or()
        } else {
            SupportedCurrency::Redgold
        };
        let fulfilled = CurrencyAmount::from_fractional_cur(fulfilled_amt_cur, fulfilled_cur).ok();
        if fulfilled.is_none() {
            return None
        }
        let f = fulfilled.unwrap();
        let vol = if from_rdg {
            self.pair_quote_volume.clone()
        } else {
            self.base_volume.clone()
        };

        if f >= vol {
            return None
        }
        let fee = PartyEvents::expected_fee_amount(fulfilled_cur, network).unwrap();

        if f < fee || f.to_fractional() <= 0.0f64 {
            return None
        }

        let of = OrderFulfillment {
            order_amount,
            fulfilled_amount: (f.to_fractional() * 1e8) as u64,
            is_ask_fulfillment_from_external_deposit: is_ask,
            event_time,
            tx_id_ref: tx_id.clone(),
            destination: destination.clone(),
            is_stake_withdrawal: false,
            stake_withdrawal_fulfilment_utxo_id: None,
            primary_event,
            prior_related_event: None,
            successive_related_event: None,
            fulfillment_txid_external: None,
            order_amount_typed,
            fulfilled_amount_typed: f.clone(),
        };
        Some(of)
    }

    fn old_fulfill(&self, order_amount_typed: CurrencyAmount, order_amount: u64, is_ask: bool, event_time: i64, tx_id: Option<ExternalTransactionId>, destination: &Address, primary_event: AddressEvent, network: &NetworkEnvironment) -> Option<OrderFulfillment> {
        let mut remaining_order_amount = order_amount.clone();
        let mut fulfilled_amount: u64 = 0;
        let mut pv_curve = if is_ask {
            // Asks are ordered in increasing amount(USD), denominated in RDG/PAIR
            self.asks()
        } else {
            // Bids are ordered in decreasing amount(USD), denominated in RDG/PAIR
            self.bids()
        };

        for pv in pv_curve.iter_mut() {
            // This price is always in RDG/PAIR
            let price_rdg_pair = pv.price;
            // This volume is RDG if ask, PAIR if bid
            let this_volume = pv.volume;

            // This is the total amount of the other currency we're swapping for that remains
            // to fulfill
            let other_amount_requested = if is_ask {
                // If we're fulfilling an ask, we're giving RDG in exchange for receiving PAIR
                // RDG / PAIR now
                // PAIR * (RDG / PAIR) = RDG
                remaining_order_amount as f64 * price_rdg_pair
            } else {
                // If we're fulfilling an bid, we're giving PAIR in exchange for receiving RDG
                // RDG / RDG/PAIR = PAIR
                remaining_order_amount as f64 / price_rdg_pair
            } as u64;

            // We need to use multiple price volume buckets to fulfill this order.
            if other_amount_requested >= this_volume {
                // We have more Other than this ask can fulfill, so we take it all and move on.
                fulfilled_amount += this_volume;
                let this_remove_from_order = if is_ask {
                    // If ask, this_volume is of unit RDG / (RDG/PAIR) = PAIR
                    this_volume as f64 / price_rdg_pair
                } else {
                    // RDG / RDG/PAIR = PAIR
                    this_volume as f64 * pv.price
                } as u64;
                // This is to catch float rounding issues
                if this_remove_from_order > remaining_order_amount {
                    break
                } else {
                    remaining_order_amount -= this_remove_from_order;
                    // Continue iterating to more price volume buckets.
                }
            } else {
                // We have less Other than this ask can fulfill, so we take it and stop
                // remaining_order_amount = 0;
                fulfilled_amount += other_amount_requested;
                break
            }
        };

        // updated_curve.retain(|v| v.volume > 0);
        let cur = destination.currency_or();
        let fee = PartyEvents::expected_fee_amount(cur, network).ok_msg("fee").expect("invalid currency in fulfill order").amount_i64_or();

        if fulfilled_amount < fee as u64 || fulfilled_amount <= 0 {
            None
        } else {
            Some(OrderFulfillment {
                order_amount,
                fulfilled_amount,
                is_ask_fulfillment_from_external_deposit: is_ask,
                event_time,
                tx_id_ref: tx_id.clone(),
                destination: destination.clone(),
                is_stake_withdrawal: false,
                stake_withdrawal_fulfilment_utxo_id: None,
                primary_event,
                prior_related_event: None,
                successive_related_event: None,
                fulfillment_txid_external: None,
                order_amount_typed,
                fulfilled_amount_typed: Default::default(),
            })
        }
    }

    pub fn recalculate_no_quote_price_change(
        existing: HashMap<SupportedCurrency, CentralPricePair>,
        reserve_volumes: HashMap<SupportedCurrency, CurrencyAmount>,
        time: i64,
    ) -> RgResult<HashMap<SupportedCurrency, CentralPricePair>> {
        let hm = existing.iter()
            .map(|(k, v)| (k.clone(), v.pair_quote_price_estimate))
            .collect();
        Self::calculate_central_prices_bid_ask(
            hm,
            reserve_volumes,
            time,
            None,
            None
        )
    }

    pub fn calculate_central_prices_bid_ask(
        external_prices_quote_pair: HashMap<SupportedCurrency, f64>,
        reserve_volumes: HashMap<SupportedCurrency, CurrencyAmount>,
        time: i64,
        enforced_base_min_usd: Option<f64>,
        bid_scale_factor: Option<f64>,
    ) -> RgResult<HashMap<SupportedCurrency, CentralPricePair>> {

        let enforced_base_min_usd = enforced_base_min_usd.unwrap_or(100.0);
        let bid_scale_factor = bid_scale_factor.unwrap_or(1.1);


        let mut ret = HashMap::new();
        let core_vol = reserve_volumes.get(&SupportedCurrency::Redgold);
        if core_vol.is_none() {
            return Ok(ret);
        }
        let core_vol = core_vol.unwrap().clone();

        for (currency, vol) in reserve_volumes.iter() {
            if currency != &SupportedCurrency::Redgold {
                // This assumes 'base_pair' is USD
                // The price from external oracle source denominated in USD.
                let quote_pair_usd_price = external_prices_quote_pair.get(currency);
                if quote_pair_usd_price.is_none() {
                    continue;
                }
                let quote_pair_usd_price = quote_pair_usd_price.unwrap().clone();

                // Let's assume price ~ $100USD/RDG
                // BTC $60kUSD/BTC
                // ETH $3kETH/USD
                // Then enforced_base_min_usd is 100, the min USD price per RDG below which we aren't dropping.

                // This value is denominated in RDG/PAIR, i.e. RDG/BTC, RDG/ETH for easier reading
                // We'd expect it to be 600 RDG/BTC, or 30 RDG/ETH if the ratio matches expected $100USD/RDG
                let reserve_ratio_rdg_pair_as_price = core_vol.to_fractional() / vol.to_fractional();

                // (USD / BTC) / (RDG / BTC) = USD / RDG
                let ratio_usd_rdg_price = quote_pair_usd_price / reserve_ratio_rdg_pair_as_price;

                let ask_adjusted_ratio_usd = if ratio_usd_rdg_price < enforced_base_min_usd {
                    // Enforce the min ask here
                    enforced_base_min_usd
                } else {
                    ratio_usd_rdg_price
                };

                // (USD / RDG) => ( RDG / USD ) * USD / BTC = RDG / BTC
                let ask_adjusted_ratio_rdg_pair = (1.0/ask_adjusted_ratio_usd) * quote_pair_usd_price;

                let bid_adjusted = bid_scale_factor*ask_adjusted_ratio_rdg_pair;
                // Same logic as ask conversion.
                let bid_adjusted_usd = quote_pair_usd_price / bid_adjusted;

                let cpp = CentralPricePair {
                    min_ask: ask_adjusted_ratio_rdg_pair,
                    min_ask_estimated: ask_adjusted_ratio_usd,
                    min_bid: bid_adjusted,
                    min_bid_estimated: bid_adjusted_usd,
                    time,
                    base_currency: SupportedCurrency::Redgold,
                    pair_quote_currency: currency.clone(),
                    pricing_estimate_pair: SupportedCurrency::Usd,
                    reserve_ratio_pair: reserve_ratio_rdg_pair_as_price,
                    base_volume: core_vol.clone(),
                    pair_quote_volume: vol.clone(),
                    pair_quote_price_estimate: quote_pair_usd_price,
                };
                ret.insert(currency.clone(), cpp);
            }
        }

        Ok(ret)
    }

}


#[test]
fn debug_calculate_sample_prices() {

    let cpp = CentralPricePair::calculate_central_prices_bid_ask(
        [
            (SupportedCurrency::Bitcoin, 60000.0),
            (SupportedCurrency::Ethereum, 3000.0),
        ].iter().cloned().collect(),
        [
            (SupportedCurrency::Redgold, CurrencyAmount::from_fractional(100.0).expect("")),
            (SupportedCurrency::Bitcoin, CurrencyAmount::from_btc(50_000)),
            (SupportedCurrency::Ethereum, CurrencyAmount::from_eth_bigint_string("055551508594791676")),
        ].iter().cloned().collect(),
        1000,
        None,
        None
    ).unwrap();

    for (k, v) in cpp.iter() {
        println!("{:?}: {:?}", k, v.json_or());
        v.bids().iter().enumerate().max_by(|a, b|
            a.1.volume.partial_cmp(&b.1.volume).unwrap()).map(|(i, v)| {
            println!("Max bid: {:?} at index {:?}", v, i);
        });
        // println!("Asks: {:?}", v.asks());
        println!("Asks_usd: {:?}", v.asks_usd());
        // println!("bids: {:?}", v.bids());
        // println!("bids_usd: {:?}", v.bids_usd());
    }

    let bpp = cpp.get(&SupportedCurrency::Bitcoin).unwrap();
    // let f = bpp.fulfill_taker_order(
    //     10_000, false, 1000, None, &Address::default(), AddressEvent::External(ExternalTimedTransaction::default()),
    //     &NetworkEnvironment::Dev
    // ).unwrap();
    // let fra = f.fulfilled_currency_amount().to_fractional();
    // println!("Fulfillment: {}", f.json_pretty_or());
    // println!("Fulfillment amount: {}", fra);
}
