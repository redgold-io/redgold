use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use redgold_schema::{RgResult, SafeOption};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::structs::{Address, CurrencyAmount, ExternalTransactionId, SupportedCurrency};
use crate::party::order_fulfillment::OrderFulfillment;
use crate::party::party_stream::PartyEvents;
use crate::party::price_volume::PriceVolume;

pub const DUST_LIMIT: i64 = 2500;

#[derive(Clone, Serialize, Deserialize)]
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
            -1.0*self.min_ask,
            self.min_ask * 3.0
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

    pub fn fulfill_taker_order(
        &self,
        order_amount: u64,
        is_ask: bool,
        event_time: i64,
        tx_id: Option<ExternalTransactionId>,
        destination: &Address
    ) -> Option<OrderFulfillment> {
        let mut remaining_order_amount = order_amount.clone();
        let mut fulfilled_amount: u64 = 0;
        let mut updated_curve = if is_ask {
            // Asks are ordered in increasing amount(USD), denominated in BTC/RDG
            // now changed to RDG/BTC
            self.asks()
        } else {
            // Bids are ordered in decreasing amount(USD), denominated in RDG/BTC
            self.bids()
        };

        for pv in updated_curve.iter_mut() {

            let other_amount_requested = if is_ask {
                // Comments left here for clarity even if code is the same
                let price = pv.price;
                // RDG / BTC now
                // BTC * (RDG / BTC) = RDG
                remaining_order_amount as f64 * price
            } else {
                // RDG / RDG/BTC = BTC
                remaining_order_amount as f64 / pv.price
            } as u64;

            let this_vol = pv.volume;
            if other_amount_requested >= this_vol {
                // We have more Other than this ask can fulfill, so we take it all and move on.
                fulfilled_amount += this_vol;
                remaining_order_amount -= (this_vol as f64 * pv.price) as u64;
                pv.volume = 0;
            } else {
                // We have less Other than this ask can fulfill, so we take it and stop
                pv.volume -= other_amount_requested;
                remaining_order_amount = 0;
                fulfilled_amount += other_amount_requested;
                break
            }
        };

        // updated_curve.retain(|v| v.volume > 0);
        let cur = destination.currency_or();
        let fee = PartyEvents::expected_fee_amount(cur).ok_msg("fee").expect("invalid currency in fulfill order").amount_i64_or();

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
            (SupportedCurrency::Redgold, CurrencyAmount::from_fractional(10.0).expect("")),
            (SupportedCurrency::Bitcoin, CurrencyAmount::from_btc(100_000)),
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
        println!("Asks: {:?}", v.asks());
        println!("Asks_usd: {:?}", v.asks_usd());
        println!("bids: {:?}", v.bids());
        println!("bids_usd: {:?}", v.bids_usd());
    }
}