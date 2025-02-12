use crate::helpers::with_metadata_hashable::WithMetadataHashable;
use crate::party::address_event::AddressEvent::External;
use crate::party::address_event::AddressEvent;
use crate::party::central_price::CentralPricePair;
use crate::party::portfolio::PortfolioRequestEvents;
use crate::structs::{Address, CurrencyAmount, DepositRequest, ExternalTransactionId, NetworkEnvironment, PublicKey, StakeDeposit, SupportedCurrency, Transaction, UtxoId};
use crate::tx::external_tx::ExternalTimedTransaction;
use crate::RgResult;
use itertools::Itertools;
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use strum_macros::{EnumIter, EnumString};


#[derive(Serialize, Deserialize, EnumString, Clone, PartialEq, Debug, EnumIter)]
pub enum AddressEventExtendedType {
    StakeDeposit, StakeWithdrawal, Swap, SwapFulfillment, Send, Receive
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PartyEvents where {
    pub network: NetworkEnvironment,
    pub party_addresses: HashMap<SupportedCurrency, Vec<Address>>,
    // pub key_address: Address,
    // pub party_public_key: PublicKey,
    pub events: Vec<AddressEvent>,
    pub balance_map: HashMap<SupportedCurrency, CurrencyAmount>,
    pub balance_pending_order_deltas_map: HashMap<SupportedCurrency, CurrencyAmount>,
    pub balance_with_deltas_applied: HashMap<SupportedCurrency, CurrencyAmount>,
    pub unfulfilled_rdg_orders: Vec<(OrderFulfillment, AddressEvent)>,
    pub unfulfilled_external_withdrawals: Vec<(OrderFulfillment, AddressEvent)>,
    // pub price: f64,
    // pub bid_ask: BidAsk,
    pub unconfirmed_events: Vec<AddressEvent>,
    // TODO: populate
    pub fulfillment_history: Vec<(OrderFulfillment, AddressEvent, AddressEvent)>,
    pub event_fulfillment: Option<OrderFulfillment>,
    pub internal_staking_events: Vec<InternalStakeEvent>,
    pub external_staking_events: Vec<ConfirmedExternalStakeEvent>,
    pub pending_stake_withdrawals: Vec<PendingWithdrawalStakeEvent>,
    pub pending_external_staking_txs: Vec<PendingExternalStakeEvent>,
    // pub pending_stake_withdrawals: Vec<WithdrawalStakingEvent>,
    pub rejected_stake_withdrawals: Vec<AddressEvent>,
    pub central_prices: HashMap<SupportedCurrency, CentralPricePair>,
    pub central_price_history: Option<Vec<(i64, HashMap<SupportedCurrency, CentralPricePair>)>>,
    // This needs to be populated if deserializing.
    // #[serde(skip)]
    // pub relay: Option<Relay>,
    pub locally_fulfilled_orders: Vec<OrderFulfillment>,
    pub portfolio_request_events: PortfolioRequestEvents,
    pub default_fee_addrs: Vec<Address>,
    pub seeds: Vec<PublicKey>,
    // pub party_pk_all_address: Vec<Address>,
}

impl PartyEvents {

    pub fn event_counts(&self) -> HashMap<SupportedCurrency, i64> {
        let mut map = HashMap::new();
        for e in self.events.iter() {
            let cur = e.currency();
            let count = map.get(&cur).cloned().unwrap_or(0);
            map.insert(cur, count + 1);
        }
        map
    }

    pub fn get_rdg_max_bid_usd_estimate_at(&self, time: i64) -> Option<f64> {
        self.central_price_history.clone().unwrap_or_default().iter().filter(|(t, _)| *t <= time).last().and_then(|(_, cp)| {
            let max = cp.iter().map(|(_, p)| {
                p.min_bid_estimated
            }).reduce(|a, b| if a > b { a } else { b });
            max
        })
    }

    pub fn find_fulfillment_of(&self, identifier: String) -> Option<(OrderFulfillment, AddressEvent, AddressEvent)> {
        self.fulfillment_history.iter().find_map(|(of, e, e2)| {
            if e.identifier() == identifier {
                Some((of.clone(), e.clone(), e2.clone()))
            } else {
                None
            }
        })
    }

    pub fn determine_event_type(&self, identifier: &String) -> Option<AddressEventExtendedType> {
        if self.find_fulfillment_of(identifier.clone()).is_some() {
            // This event was fulfilled by something, therefore it was originally a swap.
            Some(AddressEventExtendedType::Swap)
        } else if self.find_request_fulfilled_by(identifier.clone()).is_some() {
            Some(AddressEventExtendedType::SwapFulfillment)
        } else {
            None
        }
    }

    pub fn find_request_fulfilled_by(&self, identifier: String) -> Option<(OrderFulfillment, AddressEvent, AddressEvent)> {
        self.fulfillment_history.iter().find_map(|(of, e, e2)| {
            if e2.identifier() == identifier {
                Some((of.clone(), e.clone(), e2.clone()))
            } else {
                None
            }
        })
    }

    pub fn address_for_currency(&self, cur: &SupportedCurrency) -> Option<Address> {
        self.party_addresses.get(cur).and_then(|a| {
            a.last()
        }).cloned()
    }

    pub fn all_party_address(&self) -> Vec<Address> {
        self.party_addresses.clone().into_iter().flat_map(|(a,v)| {
            v
        }).collect()
    }

    pub fn key_address(&self) -> Option<Address> {
        self.party_addresses.get(&SupportedCurrency::Redgold).and_then(|a| {
            a.last()
        }).cloned()
    }

    pub fn staking_balances(&self,
                            addrs: &Vec<Address>,
                            include_amm: Option<bool>,
                            include_portfolio: Option<bool>,
    ) -> HashMap<SupportedCurrency, CurrencyAmount> {
        let has_address_filter = !addrs.is_empty();
        let include_amm = include_amm.unwrap_or(true);
        let include_portfolio = include_portfolio.unwrap_or(true);
        let str_addrs = addrs.iter().map(|a| a.render_string().unwrap()).collect::<HashSet<String>>();
        let port_events = self.portfolio_request_events.stake_utxos.iter().map(|e| e.1.clone()).collect::<Vec<ConfirmedExternalStakeEvent>>();
        let mut map = HashMap::new();
        for e in self.external_staking_events.iter() {
            if !str_addrs.contains(&e.ett.other_address) && has_address_filter {
                continue
            }
            if port_events.contains(e) && !include_portfolio {
                continue
            }

            let amt = e.ett.currency_amount();
            let cur = map.get(&e.ett.currency).cloned().unwrap_or(CurrencyAmount::zero(e.ett.currency.clone()));
            let new = cur + amt;
            map.insert(e.ett.currency.clone(), new);
        }
        if include_amm {
            for e in self.internal_staking_events.iter() {
                if !addrs.contains(&e.withdrawal_address) && has_address_filter {
                    continue
                }
                let amt = e.amount.clone();
                let cur = map.get(&e.amount.currency_or()).cloned().unwrap_or(CurrencyAmount::zero(e.amount.currency_or()));
                let new = cur + amt;
                map.insert(e.amount.currency_or(), new);
            }
        }

        map
    }

    pub fn balances_with_deltas_sub_portfolio(&self) -> HashMap<SupportedCurrency, CurrencyAmount> {
        let mut map = self.balance_with_deltas_applied.clone();
        for (amt, v) in self.portfolio_request_events.external_stake_balance_deltas.iter() {
            if let Some(cur) = map.get(&amt) {
                let new = cur.clone() - v.clone();
                map.insert(amt.clone(), new);
            }
        }
        map
    }

    pub fn num_internal_events(&self) -> usize {
        self.events.iter().filter(|e| match e {
            External(_) => { false }
            AddressEvent::Internal(_) => { true}
        }).count()
    }

    pub fn num_external_events(&self) -> usize {
        self.events.iter().filter(|e| match e {
            External(_) => { true }
            AddressEvent::Internal(_) => { false}
        }).count()
    }

    pub fn num_external_incoming_events(&self) -> usize {
        self.events.iter().filter(|e| match e {
            External(e) => { e.incoming }
            AddressEvent::Internal(_) => { false}
        }).count()
    }

    pub fn modify_pending_balance_only(&mut self, delta: CurrencyAmount) {
        let currency = delta.currency_or();
        let current = self.balance_pending_order_deltas_map
            .get(&currency).cloned().unwrap_or(CurrencyAmount::zero(currency));
        let new = current + delta.clone();
        self.balance_pending_order_deltas_map.insert(currency, new);
        // self.modify_balance_with_deltas(delta);
    }

    pub fn modify_pending_and_deltas(&mut self, delta: CurrencyAmount) {
        self.modify_pending_balance_only(delta.clone());
        self.modify_balance_with_deltas(delta);
    }


    pub fn modify_balance_with_deltas(&mut self, delta: CurrencyAmount) {
        let currency = delta.currency_or();
        let current = self.balance_with_deltas_applied
            .get(&currency).cloned().unwrap_or(CurrencyAmount::zero(currency));
        let new = current + delta;
        self.balance_with_deltas_applied.insert(currency, new);
    }

    pub fn modify_base_balance_and_deltas(&mut self, delta: CurrencyAmount) {
        let currency = delta.currency_or();
        let current = self.balance_map
            .get(&currency).cloned().unwrap_or(CurrencyAmount::zero(currency));
        let new = current + delta.clone();
        self.balance_map.insert(currency, new);
        self.modify_balance_with_deltas(delta);
    }

    pub fn unconfirmed_rdg_output_btc_txid_refs(&self) -> HashSet<String> {
        self.unconfirmed_events.iter().filter_map(|e| {
            match e {
                AddressEvent::Internal(t) => {
                    Some(t.tx.output_external_txids().map(|t| t.identifier.clone()))
                }
                _ => {
                    None
                }
            }
        }).flatten().collect()
    }

    pub fn unconfirmed_btc_output_other_addresses(&self) -> HashSet<String> {
        let mut hs = HashSet::new();

        for e in self.unconfirmed_events.iter() {
            match e {
                AddressEvent::External(t) => {
                    if !t.incoming {
                        // This is a transaction we sent (the party) to some output address not ourself
                        // which has yet to be confirmed, but we don't want to duplicate it.
                        t.other_output_addresses.iter().for_each(|a| {
                            hs.insert(a.clone());
                        });
                    }
                }
                _ => {
                }
            }
        }
        hs
    }

    pub fn unconfirmed_identifiers(&self) -> HashSet<String> {
        let ids = self.unconfirmed_events.iter().map(|d| d.identifier())
            .collect::<HashSet<String>>();
        ids
    }

    // pub fn process_locally_fulfilled_orders(&mut self, orders: Vec<OrderFulfillment>){
    //     self.locally_fulfilled_orders.extend(orders);
    // }


    pub fn recalculate_prices(&mut self, time: i64) -> RgResult<()> {

        let prior = self.central_prices.clone();
        self.central_prices = CentralPricePair::recalculate_no_quote_price_change(
            self.central_prices.clone(),
            self.balances_with_deltas_sub_portfolio(),
            time
        )?;
        if self.central_prices != prior {
            match self.central_price_history.as_mut() {
                None => {
                    self.central_price_history = Some(vec![(time, self.central_prices.clone())]);
                }
                Some(a) => {
                    a.push((time, self.central_prices.clone()));
                }
            }
        }

        Ok(())
    }

    pub fn fulfill_order(
        &mut self,
        amount: CurrencyAmount,
        is_ask: bool,
        event_time: i64,
        tx_id: Option<ExternalTransactionId>,
        destination: &Address,
        is_stake: bool,
        event: &AddressEvent,
        stake_utxo_id: Option<UtxoId>,
        event_currency: SupportedCurrency,
        primary_event: AddressEvent,
        prior_related_event: Option<AddressEvent>
    ) -> RgResult<()> {
        let fulfillment = if !is_stake {
            let currency = if is_ask {
                amount.currency_or()
            } else {
                event_currency
            };
            if let Some(cp) = self.central_prices.get(&currency) {
                let of = cp.fulfill_taker_order(
                    amount.clone(),
                    amount.amount_i64_or() as u64, is_ask, event_time, tx_id, &destination, primary_event,
                    &self.network
                );
                if let Some(of) = of.as_ref() {
                    if is_ask {
                        self.unfulfilled_rdg_orders.push((of.clone(), event.clone()));
                    } else {
                        self.unfulfilled_external_withdrawals.push((of.clone(), event.clone()));
                    }
                }
                of
            } else {
                None
            }
        } else {
            let of = OrderFulfillment {
                order_amount_typed: amount.clone(),
                order_amount: amount.amount_i64_or() as u64,
                fulfilled_amount: amount.amount_i64_or() as u64,
                is_ask_fulfillment_from_external_deposit: false,
                event_time,
                tx_id_ref: None,
                destination: destination.clone(),
                is_stake_withdrawal: true,
                stake_withdrawal_fulfilment_utxo_id: stake_utxo_id,
                primary_event,
                prior_related_event,
                successive_related_event: None,
                fulfillment_txid_external: None,
                fulfilled_amount_typed: amount.clone(),
            };

            Some(of)
        };

        if let Some(fulfillment) = fulfillment {
            self.event_fulfillment = Some(fulfillment.clone());
            self.modify_pending_and_deltas(fulfillment.fulfilled_currency_amount() * -1);
        }
        Ok(())
    }

    pub fn retain_unfulfilled_deposits(tx_id: &ExternalTransactionId, d: &AddressEvent) -> bool {
        match d {
            AddressEvent::External(t2) => {
                let receipt_match = t2.tx_id == tx_id.identifier;
                !receipt_match
            }
            AddressEvent::Internal(_) => {
                true
            }
        }
    }

    pub fn expected_fee_amount(currency: SupportedCurrency, env: &NetworkEnvironment) -> Option<CurrencyAmount> {
        match currency {
            SupportedCurrency::Redgold => {
                Some(CurrencyAmount::from_fractional(0.0001).unwrap())
            }
            SupportedCurrency::Bitcoin => {
                let btc = if env.is_main() {
                    850
                } else {
                    2_000
                };
                Some(CurrencyAmount::from_btc(btc))
            }
            SupportedCurrency::Ethereum => {
                Some(CurrencyAmount::eth_fee_fixed_normal_by_env(env))
            }
            _ => None
        }
    }
    pub fn retain_unfulfilled_withdrawals(t: &ExternalTimedTransaction, d: &AddressEvent) -> bool {
        match d {
            AddressEvent::Internal(t2) => {
                let this_dest = t.other_address.clone().to_lowercase();
                let swap_dest = t2.tx.swap_destination();
                let swap_dest_str = swap_dest.and_then(|sd| sd.render_string().ok())
                    .map(|s| s.to_lowercase());
                // if t.currency == SupportedCurrency::Ethereum {
                //     info!("debug");
                // }
                if let Some(dest) = swap_dest_str {
                    let matching_receipt = this_dest == dest;
                    if matching_receipt {
                        return false
                    }
                }
                if let Some(sw) = t2.tx.stake_withdrawal_request().and_then(|sr| sr.destination.as_ref()).and_then(|a| a.render_string().ok()) {
                    let matching_receipt = this_dest == sw;
                    if matching_receipt {
                        return false
                    }
                }
            }
            _ => {}
        }
        true
    }


    pub fn remove_unconfirmed_event(&mut self, event: &AddressEvent) {
        self.unconfirmed_events.retain(|e| {
            match (e, event) {
                (AddressEvent::External(e), AddressEvent::External(e2)) => {
                    e.tx_id != e2.tx_id
                }
                (AddressEvent::Internal(t), AddressEvent::Internal(t2)) => {
                    t.tx.hash_or() != t2.tx.hash_or()
                }
                _ => true
            }
        })
    }

    fn minimum_swap_amount(amt: &CurrencyAmount) -> bool {
        match amt.currency_or() {
            SupportedCurrency::Redgold => {
                amt.amount >= 10000
            }
            SupportedCurrency::Bitcoin => {
                amt.amount >= 2000
            }
            SupportedCurrency::Ethereum => {
                amt.bigint_amount().map(|b| b >= BigInt::from(1e12 as i64)).unwrap_or(false)
            }
            _ => false
        }
    }

}


#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct OrderFulfillment {
    pub order_amount: u64,
    pub fulfilled_amount: u64,
    pub is_ask_fulfillment_from_external_deposit: bool,
    pub event_time: i64,
    pub tx_id_ref: Option<ExternalTransactionId>,
    pub destination: Address,
    pub is_stake_withdrawal: bool,
    pub stake_withdrawal_fulfilment_utxo_id: Option<UtxoId>,
    pub primary_event: AddressEvent,
    pub prior_related_event: Option<AddressEvent>,
    pub successive_related_event: Option<AddressEvent>,
    pub fulfillment_txid_external: Option<ExternalTransactionId>,
    pub order_amount_typed: CurrencyAmount,
    pub fulfilled_amount_typed: CurrencyAmount,
}

impl OrderFulfillment {

    pub fn fulfilled_currency_amount(&self) -> CurrencyAmount {
        let c = self.destination.currency_or();
        if c == SupportedCurrency::Ethereum {
            CurrencyAmount::from_eth_i64(self.fulfilled_amount as i64)
        } else {
            CurrencyAmount::from_currency(self.fulfilled_amount as i64, c)
        }
    }



}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InternalStakeEvent {
    pub event: AddressEvent,
    pub tx: Transaction,
    pub amount: CurrencyAmount,
    pub withdrawal_address: Address,
    pub liquidity_deposit: StakeDeposit,
    pub utxo_id: UtxoId,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct PendingExternalStakeEvent {
    pub event: AddressEvent,
    pub tx: Transaction,
    pub amount: CurrencyAmount,
    pub external_address: Address,
    pub external_currency: SupportedCurrency,
    pub liquidity_deposit: StakeDeposit,
    pub deposit_inner: DepositRequest,
    pub utxo_id: UtxoId,
}


#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct ConfirmedExternalStakeEvent {
    pub pending_event: PendingExternalStakeEvent,
    pub event: AddressEvent,
    pub ett: ExternalTimedTransaction,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct PendingWithdrawalStakeEvent {
    pub address: Address,
    pub amount: CurrencyAmount,
    pub initiating_event: AddressEvent,
    pub is_external: bool,
    pub utxo_id: UtxoId,
}
