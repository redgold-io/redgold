use std::collections::{HashMap, HashSet};
use std::ops::Sub;
use std::sync::{Arc, Mutex};
use bdk::bitcoin::hashes::hex::ToHex;
use bdk::bitcoin::psbt::PartiallySignedTransaction;
use bdk::database::BatchDatabase;
use itertools::Itertools;
use num_bigint::BigInt;
use rocket::serde::{Deserialize, Serialize};
use rocket::yansi::Paint;
use serde::__private::de::IdentifierDeserializer;
use tracing::event;
use redgold_keys::eth::eth_wallet::EthWalletWrapper;
use redgold_keys::eth::historical_client::EthHistoricalClient;
use redgold_keys::proof_support::PublicKeySupport;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::util::btc_wallet::{ExternalTimedTransaction, SingleKeyBitcoinWallet};
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::{error_info, RgResult, SafeOption};
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::output::output_data;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::seeds::get_seeds_by_env_time;
use redgold_schema::structs::{Address, CurrencyAmount, ErrorInfo, ExternalTransactionId, NetworkEnvironment, ObservationProof, PublicKey, SupportedCurrency, Transaction, UtxoId};
use redgold_schema::util::lang_util::AnyPrinter;
use crate::core::relay::Relay;
// use crate::multiparty_gg20::watcher::{get_btc_per_rdg_starting_min_ask, OrderFulfillment};
use crate::party::address_event::AddressEvent;
use crate::party::address_event::AddressEvent::External;
use crate::party::central_price::CentralPricePair;
use crate::party::data_enrichment::PartyInternalData;
use crate::party::order_fulfillment::OrderFulfillment;
use crate::party::price_query::PriceDataPointUsdQuery;
use crate::party::stake_event_stream::{ConfirmedExternalStakeEvent, InternalStakeEvent, PendingExternalStakeEvent, PendingWithdrawalStakeEvent};
use crate::util::current_time_millis_i64;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TransactionWithObservationsAndPrice {
    pub tx: Transaction,
    pub observations: Vec<ObservationProof>,
    pub price_usd: Option<f64>
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PartyEvents {
    pub(crate) network: NetworkEnvironment,
    key_address: Address,
    pub(crate) party_public_key: PublicKey,
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
    // This needs to be populated if deserializing.
    #[serde(skip)]
    pub relay: Option<Relay>,
}

impl PartyEvents {

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

    pub fn validate_rdg_swap_fulfillment_transaction(&self, tx: &Transaction) -> RgResult<()> {
        let rdg_orders = self.orders().into_iter()
            .filter(|e| e.fulfilled_currency_amount().currency_or() == SupportedCurrency::Redgold)
            .collect_vec();

        // TODO: add stake withdrawal fulfillment here too.
        for o in tx.outputs.iter() {
            if let Some(amt) = o.opt_amount_typed() {
                let a = o.address.safe_get_msg("Missing address")?;
                if self.party_public_key.to_all_addresses()?.contains(&a) {
                    continue;
                }
                if o.is_fee() && self.relay.safe_get_msg("Missing relay")?.default_fee_addrs().contains(&a) {
                    continue;
                }
                let withdrawal = rdg_orders.iter().find(|o|
                    o.is_stake_withdrawal && &o.destination == a && o.fulfilled_currency_amount() == amt);
                if withdrawal.is_some() {
                    continue;
                }
                let ful = o.swap_fulfillment();
                let f = ful.safe_get_msg("Missing swap fulfillment")?;
                let txid = f.external_transaction_id.safe_get_msg("Missing txid")?;
                let order = rdg_orders.iter()
                    .find(|o| o.tx_id_ref.as_ref() == Some(txid) && o.fulfilled_currency_amount().amount_i64_or() == amt.amount_i64_or());
                if order.is_none() {
                    return Err(error_info("Invalid fulfillment for output"))
                        .with_detail("output", o.json_or());
                }
            }
        }

        Ok(())
    }

    pub fn fulfillment_orders(&self, c: SupportedCurrency) -> Vec<OrderFulfillment> {
        self.orders().iter().filter(|o| o.destination.currency_or() == c).cloned().collect()
    }
    pub fn validate_btc_fulfillment<D: BatchDatabase>(
        &self,
        validation_payload: String,
        signing_data: Vec<u8>,
        w: &mut SingleKeyBitcoinWallet<D>
    ) -> RgResult<()> {

        let btc = self.fulfillment_orders(SupportedCurrency::Bitcoin)
            .iter()
            .map(|o| (o.destination.render_string().expect("works"), o.fulfilled_currency_amount().amount_i64_or() as u64))
            .collect_vec();

        let psbt: PartiallySignedTransaction = validation_payload.clone().json_from()?;
        w.psbt = Some(psbt.clone());
        let has_expected_hash = w.signable_hashes()?.iter().filter(|(h, _)| h == &signing_data).next().is_some();
        if !has_expected_hash {
            return Err(error_info("Invalid BTC fulfillment signing data"))
                .with_detail("signing_data", hex::encode(signing_data))
                .with_detail("payload", validation_payload);
        }

        let party_self = self.party_public_key.to_all_addresses()?.iter().flat_map(|a| a.render_string().ok()).collect_vec();
        let outs = w.convert_psbt_outputs();
        for (out_addr, out_amt) in outs {
            if party_self
                .iter()
                .find(|&a| a == &out_addr).is_some() {
                continue;
            }
            if btc.iter().find(|(addr, amt) | addr == &out_addr && *amt == out_amt).is_none() {
                return Err(error_info("Invalid BTC fulfillment output"))
                    .with_detail("output_address", out_addr)
                    .with_detail("output_amount", out_amt.to_string());
            }
        }
        Ok(())
    }

    pub fn validate_eth_fulfillment(&self, typed_tx_payload: String, signing_data: Vec<u8>) -> RgResult<()> {
        let fulfills = self.fulfillment_orders(SupportedCurrency::Ethereum)
            .into_iter()
            .map(|o| {
                (o.destination.clone(), o.fulfilled_currency_amount())
            }).collect_vec();
        EthWalletWrapper::validate_eth_fulfillment(fulfills, &typed_tx_payload, &signing_data, &self.network)?;
        Ok(())
    }
}


impl PartyEvents {

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

    pub fn orders_default_cutoff(&self) -> Vec<OrderFulfillment> {
        let cutoff_time = current_time_millis_i64() - 30_000; //
        self.orders().iter().filter(|o| o.event_time < cutoff_time).cloned().collect()
    }

    pub fn orders(&self) -> Vec<OrderFulfillment> {
        let mut orders = vec![];

        let rdg_extern_txids = self.unconfirmed_rdg_output_btc_txid_refs();

        for (of, ae) in self.unfulfilled_rdg_orders.iter() {
            match ae {
                AddressEvent::External(t) => {
                    // Since this is a BTC incoming transaction,
                    // we need to check for unconfirmed events that have the txid in one of the output refs
                    if !rdg_extern_txids.contains(&t.tx_id) {
                        orders.push(of.clone());
                    }
                }
                AddressEvent::Internal(_) => {}
            }
        }

        for (of, ae) in &self.unfulfilled_external_withdrawals {
            match ae {
                AddressEvent::Internal(t) => {
                    // Since this is a RDG incoming transaction, which we'll fulfill with BTC,
                    // We need to know it's corresponding BTC address to see if an unconfirmed output matches it
                    // (i.e. it's already been unconfirmed fulfilled.)
                    t.tx.first_input_address_to_btc_address(&self.network).map(|addr| {
                        if !self.unconfirmed_btc_output_other_addresses().contains(&addr) {
                            orders.push(of.clone());
                        }
                    });
                }
                AddressEvent::External(_) => {}
            }
        }


        orders.sort_by(|a, b| a.event_time.cmp(&b.event_time));
        orders
    }

    pub fn unconfirmed_identifiers(&self) -> HashSet<String> {
        let ids = self.unconfirmed_events.iter().map(|d| d.identifier())
            .collect::<HashSet<String>>();
        ids
    }
    pub fn new(party_public_key: &PublicKey, network: &NetworkEnvironment, relay: &Relay) -> Self {
        // let btc_rdg = get_btc_per_rdg_starting_min_ask(0);
        // let min_ask = btc_rdg;
        // let price = 1f64 / btc_rdg;
        Self {
            network: network.clone(),
            key_address: party_public_key.address().expect("works").clone(),
            party_public_key: party_public_key.clone(),
            events: vec![],
            balance_map: Default::default(),
            balance_pending_order_deltas_map: Default::default(),
            balance_with_deltas_applied: Default::default(),
            unfulfilled_rdg_orders: vec![],
            unfulfilled_external_withdrawals: vec![],
            // price,
            // bid_ask: BidAsk::generate_default(
            //     0, 0, price, min_ask
            // ),
            unconfirmed_events: vec![],
            fulfillment_history: vec![],
            event_fulfillment: None,
            internal_staking_events: vec![],
            external_staking_events: vec![],
            pending_stake_withdrawals: vec![],
            pending_external_staking_txs: vec![],
            // pending_stake_withdrawals: vec![],
            rejected_stake_withdrawals: vec![],
            central_prices: Default::default(),
            relay: Some(relay.clone())
        }
    }

    pub async fn process_event(&mut self, e: &AddressEvent) -> RgResult<()> {
        self.events.push(e.clone());
        let seeds = self.relay.safe_get_msg("Missing relay in process event")?.node_config.seeds_now_pk();
        let time = e.time(&seeds);
        if let Some(t) = time {
            self.process_confirmed_event(e, t).await?;
        } else {
            self.unconfirmed_events.push(e.clone());
        }

        Ok(())
    }

    async fn process_confirmed_event(&mut self, e: &AddressEvent, time: i64) -> Result<(), ErrorInfo> {
        let ec = e.clone().clone();
        self.event_fulfillment = None;
        // First update latest USD price oracle information
        if let (Some(p), Some(c)) = (e.usd_event_price(), e.external_currency()) {
            let mut price_input = HashMap::new();
            price_input.insert(c, p);
            let prices = CentralPricePair::calculate_central_prices_bid_ask(
                price_input,
                self.balance_with_deltas_applied.clone(),
                time,
                None,
                None
            )?;
            for (k, v) in prices {
                self.central_prices.insert(k, v);
            }
        }
        self.recalculate_prices(time)?;
        match e {
            // External Bitcoin/Ethereum/Etc. Transaction event
            AddressEvent::External(t) => {
                self.handle_external_event(e, time, &ec, t)?;
            }
            // Internal Redgold transaction event
            AddressEvent::Internal(t) => {
                self.handle_internal_event(e, time, ec, t)?;
            }
        }
        self.recalculate_prices(time)?;
        // let new_price = if let Some(f) = self.event_fulfillment.as_ref() {
        //     let p_delta = f.fulfillment_fraction();
        //     self.price * (1.0 + p_delta)
        // } else {
        //     self.price
        // };
        // let min_ask = get_btc_per_rdg_starting_min_ask(time);
        // let balance = self.balance_map.get(&SupportedCurrency::Redgold).unwrap_or(&(0i64)).clone();
        // let pair_balance = self.balance_map.get(&SupportedCurrency::Bitcoin).unwrap_or(&(0i64)).clone() as u64;

        //
        // self.bid_ask = BidAsk::generate_default(
        //     balance, pair_balance, new_price, min_ask
        // );

        // info!("New bid ask: {}", self.bid_ask.json_or());
        // info!("New balances: {}", self.balance_map.json_or());
        // self.price = new_price;
        Ok(())
    }

    pub fn recalculate_prices(&mut self, time: i64) -> RgResult<()> {
        self.central_prices = CentralPricePair::recalculate_no_quote_price_change(
            self.central_prices.clone(),
            self.balance_with_deltas_applied.clone(),
            time
        )?;
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
    ) -> RgResult<()> {
        let fulfillment = if !is_stake {
            let currency = if is_ask {
                amount.currency_or()
            } else {
                destination.currency_or()
            };
            if let Some(cp) = self.central_prices.get(&currency) {
                let of = cp.fulfill_taker_order(
                    amount.amount_i64_or() as u64, is_ask, event_time, tx_id, &destination
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
                order_amount: amount.amount_i64_or() as u64,
                fulfilled_amount: amount.amount_i64_or() as u64,
                is_ask_fulfillment_from_external_deposit: false,
                event_time,
                tx_id_ref: None,
                destination: destination.clone(),
                is_stake_withdrawal: true,
                stake_withdrawal_fulfilment_utxo_id: stake_utxo_id,
            };

            Some(of)
        };

        if let Some(fulfillment) = fulfillment {
            self.event_fulfillment = Some(fulfillment.clone());
            self.modify_pending_and_deltas(fulfillment.fulfilled_currency_amount() * -1);
        }
        Ok(())
    }

    fn handle_internal_event(&mut self, e: &AddressEvent, time: i64, ec: AddressEvent, t: &TransactionWithObservationsAndPrice) -> RgResult<()> {
        let mut amount = CurrencyAmount::from_rdg(0);
        // TODO: Does this need to be a detect on all addresses?
        let incoming = !t.tx.input_addresses().contains(&self.key_address);

        if incoming {
            amount = t.tx.output_rdg_amount_of_pk(&self.party_public_key)?;
            // Is Swap
            if let Some(swap_destination) = t.tx.swap_destination() {
                // Represents a withdrawal from network / swap initiation event
                self.fulfill_order(
                    amount.clone(), false, time, None, &swap_destination, false, e, None
                )?;
            } else if t.tx.is_stake() {
                self.handle_stake_requests(e, time, &t.tx)?;
                // Represents a stake deposit initiation event OR just a regular transaction sending here
                // TODO: Don't match this an else, but rather allow both swaps and stakes as part of the same TX.
            }
        } else {
            let outgoing_amount = t.tx.output_rdg_amount_of_exclude_pk(&self.party_public_key)?;
            amount = outgoing_amount;
            // This is an outgoing transaction representing a deposit fulfillment receipt
            for tx_id in t.tx.output_external_txids() {
                self.remove_unconfirmed_event(e);
                let mut found_match = false;
                self.unfulfilled_rdg_orders.retain(|(of, d)| {
                    let res = Self::retain_unfulfilled_deposits(tx_id, d);
                    if !res {
                        let fulfillment = (of.clone(), d.clone(), ec.clone());
                        self.fulfillment_history.push(fulfillment);
                        found_match = true;
                    }
                    res
                });
                if found_match {
                    self.modify_pending_and_deltas(amount.clone());
                    break;
                }
                // info!("Outgoing RDG tx fulfillment for BTC tx_id: {} {}", tx_id.identifier.clone(), t.tx.json_or());
            }
            for f in t.tx.stake_withdrawal_fulfillments() {
                if let Some(utxo_id) = f.stake_withdrawal_request.as_ref() {
                    let mut found_match = false;
                    self.unfulfilled_rdg_orders.retain(|(of, d)| {
                        match d {
                            External(_) => { true }
                            AddressEvent::Internal(tx) => {
                                let res = tx.tx.input_utxo_ids().collect_vec().contains(&utxo_id);
                                if !res {
                                    let fulfillment = (of.clone(), d.clone(), ec.clone());
                                    self.fulfillment_history.push(fulfillment);
                                    found_match = true;
                                }
                                res
                            }
                        }
                    });

                if found_match {
                    self.modify_pending_and_deltas(amount.clone());
                    break;
                }
                }
            }
        }
        let delta = if incoming {
            amount
        } else {
            amount * -1
        };
        self.modify_base_balance_and_deltas(delta);
        Ok(())
    }

    fn check_external_event_expected(&mut self, ev: &AddressEvent) -> bool {
        // TODO: add other expected types here.
        self.check_external_event_pending_stake(ev)
    }

    fn handle_external_event(&mut self, e: &AddressEvent, time: i64, ec: &AddressEvent, t: &ExternalTimedTransaction) -> RgResult<()> {

        if t.incoming {

            // First check if this matches a pending stake event.
            if !self.check_external_event_expected(e) {

                // Then assume this is a deposit/swap ASK requested fulfillment
                let mut other_addr = t.other_address_typed().expect("addr");
                // Since this is an ask fulfillment, we are receiving some external event currency
                // And using the address from that deposit as the fulfillment address denominated in
                // Redgold.
                other_addr.currency = SupportedCurrency::Redgold as i32;

                let mut extid = ExternalTransactionId::default();
                extid.identifier = t.tx_id.clone();
                extid.currency = t.currency as i32;
                self.fulfill_order(
                    t.currency_amount(), true, time, Some(extid), &other_addr, false, &e, None
                )?;
            // Represents a deposit / swap external event.
            // This should be a fulfillment of an ASK, corresponding to a TAKER BUY
            // Corresponding to a price increase
            // Event initiator, has no pairing event yet (short of staking requests)
            // Balance / price adjustment event

            // Expect BTC here

            }
        } else {
            // Represents a receipt transaction for outgoing swap / stake event / withdrawal.
            // Should have a paired internal deposit event
            let mut found_match = false;

            self.unfulfilled_external_withdrawals.retain(|(of, d)| {
                let res = Self::retain_unfulfilled_withdrawals(t, d);
                if !res {
                    // This represents and outgoing BTC fulfillment of an incoming RDG tx
                    let fulfillment = (of.clone(), d.clone(), ec.clone());
                    self.fulfillment_history.push(fulfillment);
                    found_match = true;
                    // info!("Outgoing BTC tx fulfillment with hash: {} to {} fulfillment {}", t.tx_id.clone(), t.other_address, of.json_or());
                };
                res
            });


            if found_match {
                self.modify_pending_and_deltas(t.balance_change());
            }

            self.remove_unconfirmed_event(&e);
            // info!("Outgoing BTC tx {}", t.json_or());
        }
        let delta = t.balance_change();
        let delta = if t.incoming {
            delta
        } else {
            delta * -1
        };
        self.modify_base_balance_and_deltas(delta);
        Ok(())
    }

    fn retain_unfulfilled_deposits(tx_id: &ExternalTransactionId, d: &AddressEvent) -> bool {
        match d {
            AddressEvent::External(t2) => {
                let receipt_match = t2.tx_id == tx_id.identifier;
                !receipt_match
            }
            AddressEvent::Internal(i) => {
                true
            }
        }
    }

    fn retain_unfulfilled_withdrawals(t: &ExternalTimedTransaction, d: &AddressEvent) -> bool {
        match d {
            AddressEvent::Internal(t2) => {
                let this_outgoing_destination = t.other_address_typed().ok();
                if let Some(this_dest) = this_outgoing_destination {
                    if let Some(dest) = t2.tx.swap_destination() {
                        let matching_receipt = &this_dest == dest;
                        if !matching_receipt {
                            return false
                        }
                    }
                    if let Some(sw) = t2.tx.stake_withdrawal_request().and_then(|sr| sr.destination.as_ref()) {
                        let matching_receipt = &this_dest == sw;
                        if !matching_receipt {
                            return false
                        }
                    }
                }
            }
            _ => {}
        }
        true
    }


    fn remove_unconfirmed_event(&mut self, event: &AddressEvent) {
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
//
// #[ignore]
// #[tokio::test]
// async fn debug_event_stream() {
//     debug_events().await.unwrap();
// }
// async fn debug_events() -> RgResult<()> {
//
//
//     let pk_hex = "03879516077881c5be714024099c16974910d48b691c94c1824fad9635c17f3c37";
//     let pk_address = PublicKey::from_hex(pk_hex).expect("pk");
//
//     let relay = Relay::dev_default().await;
//
//     let btc_wallet =
//     Arc::new(Mutex::new(
//         SingleKeyBitcoinWallet::new_wallet(pk_address.clone(), NetworkEnvironment::Dev, true)
//             .expect("w")));
//
//     let n = PartyEvents::historical_initialize(&pk_address, &relay, &btc_wallet).await?;
//
//
//     let mut txids = HashSet::new();
//     let mut txidsbtc = HashSet::new();
//
//     for e in &n.events {
//         match e {
//             AddressEvent::External(t) => {
//                 if t.incoming {
//                     txidsbtc.insert(t.tx_id.clone());
//                 }
//             }
//             AddressEvent::Internal(int) => {
//                 if let Some(txid) = int.tx.first_output_external_txid() {
//                     txids.insert(txid.identifier.clone());
//                 }
//             }
//         }
//     };
//
//     let _missing = txidsbtc.sub(&txids);
//
//     // transactions
//     //
//     // let seeds = relay.node_config.seeds.iter().flat_map(|s| s.public_key.clone()).collect_vec();
//     //
//     // // First get all transactions associated with the address, both incoming or outgoing.
//     //
//     // let txf = relay.ds.transaction_store
//     //     .get_filter_tx_for_address(&n.key_address, 10000, 0, true).await?;
//     //
//     // let tx = relay.ds.transaction_store
//     //     .get_all_tx_for_address(&n.key_address, 100000, 0).await?;
//     //
//     // let mut res = vec![];
//     // for t in tx {
//     //     let h = t.hash_or();
//     //     let obs = relay.ds.observation.select_observation_edge(&h).await?;
//     //     let txo = TransactionWithObservations {
//     //         tx: t,
//     //         observations: obs,
//     //     };
//     //     let ae = AddressEvent::Internal(txo);
//     //     res.push(ae);
//     // }
//
//     // btc_wallet.lock().map_err(|e| error_info(format!("Failed to lock wallet: {}", e).as_str()))?
//     //     .get_all_tx()?.iter().for_each(|t| {
//     //     let ae = AddressEvent::External(t.clone());
//     //     res.push(ae);
//     // });
//     //
//     // res.sort_by(|a, b| a.time(&seeds).cmp(&b.time(&seeds)));
//     //
//     // n.events = res.clone();
//     //
//     // for e in &res {
//     //     n.process_event(e).await?;
//     // }
//
//
//     let _orders = n.orders();
//
//     Ok(())
//
//     // DepositWatcher::get_starting_center_price_rdg_btc_fallback()
//
// }


#[ignore]
#[tokio::test]
async fn debug_event_stream2() {
    crate::party::party_stream::debug_events2().await.unwrap();
}
async fn debug_events2() -> RgResult<()> {


    let relay = Relay::dev_default().await;
    relay.ds.run_migrations().await?;

    let res = relay.ds.multiparty_store.all_party_info_with_key().await?;
    let pi = res.get(0).expect("head");

    let key = pi.party_key.clone().expect("key");
    let data = relay.ds.multiparty_store.party_data(&key).await.expect("data")
        .and_then(|pd| pd.json_party_internal_data)
        .and_then(|pid| pid.json_from::<PartyInternalData>().ok()).expect("pid");

    let pev = data.party_events.clone().expect("v");

    // pev.json_pretty_or().print();
    // not this

    let cent = pev.central_prices.get(&SupportedCurrency::Bitcoin).expect("redgold");

        cent.json_pretty_or().print();
    cent.fulfill_taker_order(10_000, true, 1722524343044, None, &Address::default()).json_pretty_or().print();
    Ok(())
    // let pk_hex = "024cfc97a479af32fcb9d7b59c0e1273832817bf0bb264227e56e449d1a6b30e8e";
    // let pk_address = PublicKey::from_hex_direct(pk_hex).expect("pk");
    //
    // let eth_addr = "0x7D464545F9E9E667bbb1A907121bccb49Dc39160".to_string();
    // let eth = EthHistoricalClient::new(&NetworkEnvironment::Dev).expect("").expect("");
    // let tx = eth.get_all_tx(&eth_addr, None).await.expect("");
    //
    // let mut events = vec![];
    // for e in &tx {
    //     events.push(External(e.clone()));
    // };
    //
    // let mut pq = PriceDataPointUsdQuery::new();
    // pq.enrich_address_events(&mut events, &relay.ds).await.expect("works");
    //
    // let mut pe = PartyEvents::new(&pk_address, &NetworkEnvironment::Dev, &relay);
    //
    //
    // for e in &events {
    //
    //     pe.process_event(e).await?;
    // }
    //
    //
    // println!("{}", pe.json_or());
    //
    // Ok(())

}