use crate::core::relay::Relay;
use crate::party::portfolio_request::PortfolioEventMethods;
use crate::party::stake_event_stream::StakeMethods;
use itertools::Itertools;
use redgold_keys::external_tx_support::ExternalTxSupport;
use redgold_keys::proof_support::PublicKeySupport;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::party::address_event::AddressEvent::External;
use redgold_schema::party::address_event::{AddressEvent, TransactionWithObservationsAndPrice};
use redgold_schema::party::central_price::CentralPricePair;
use redgold_schema::party::party_events::{OrderFulfillment, PartyEvents};
use redgold_schema::structs::{Address, CurrencyAmount, ErrorInfo, ExternalTransactionId, NetworkEnvironment, PublicKey, SupportedCurrency, Transaction};
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use redgold_schema::util::times::current_time_millis;
use redgold_schema::{error_info, RgResult, SafeOption};
use std::collections::HashMap;
use log::info;

pub trait PartyEventBuilder {
    fn new(network: &NetworkEnvironment, relay: &Relay, addresses: HashMap<SupportedCurrency, Vec<Address>>) -> Self;
    fn orders(&self) -> Vec<OrderFulfillment>;
    fn fulfillment_orders(&self, c: SupportedCurrency) -> Vec<OrderFulfillment>;
    fn handle_internal_event(&mut self, e: &AddressEvent, time: i64, ec: AddressEvent, t: &TransactionWithObservationsAndPrice) -> RgResult<()>;
    async fn process_confirmed_event(&mut self, e: &AddressEvent, time: i64) -> Result<(), ErrorInfo>;
    async fn process_event(&mut self, e: &AddressEvent) -> RgResult<()>;
    fn check_external_event_expected(&mut self, ev: &AddressEvent) -> bool;
    fn handle_external_event(&mut self, e: &AddressEvent, time: i64, ec: &AddressEvent, t: &ExternalTimedTransaction) -> RgResult<()>;
}

impl PartyEventBuilder for PartyEvents {

    fn check_external_event_expected(&mut self, ev: &AddressEvent) -> bool {
        // TODO: add other expected types here.
        self.check_external_event_pending_stake(ev)
    }

    fn handle_external_event(&mut self, e: &AddressEvent, time: i64, ec: &AddressEvent, t: &ExternalTimedTransaction) -> RgResult<()> {

        if ec.incoming() {

            // First check if this matches a pending stake event.
            if !self.check_external_event_expected(e) {

                // Then assume this is a swap for external pair to RDG.
                let mut other_addr = t.other_address_typed().expect("addr");
                // Since this is an ask fulfillment, we are receiving some external event currency
                // And using the address from that deposit as the fulfillment address denominated in
                // Redgold.
                other_addr.currency = SupportedCurrency::Redgold as i32;

                let mut extid = ExternalTransactionId::default();
                extid.identifier = t.tx_id.clone();
                extid.currency = t.currency as i32;
                self.fulfill_order(
                    t.currency_amount(), true, time, Some(extid), &other_addr, false, &e, None, t.currency,
                    ec.clone(),
                    None
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
            // if t.currency == SupportedCurrency::Ethereum {
            //     info!("Eth outgoing");
            // }

            self.unfulfilled_internal_tx_requiring_external_outgoing_mpc_withdrawals.retain(|(of, d)| {
                let res = Self::retain_unfulfilled_withdrawals(t, d);
                if !res {
                    // This represents and outgoing BTC fulfillment of an incoming RDG tx
                    let fulfillment = (of.clone(), d.clone(), ec.clone());
                    self.fulfillment_history.push(fulfillment.clone());
                    found_match = true;
                    // info!("Outgoing BTC tx fulfillment with hash: {} to {} fulfillment {}", t.tx_id.clone(), t.other_address, of.json_or());
                };
                res
            });


            if found_match {
                let fulfillment = self.fulfillment_history.last().unwrap().clone();
                self.handle_maybe_portfolio_stake_withdrawal_event(fulfillment, t.clone());
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

    async fn process_event(&mut self, e: &AddressEvent) -> RgResult<()> {
        self.events.push(e.clone());
        let seeds = self.seeds.clone();
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


    fn handle_internal_event(&mut self, e: &AddressEvent, time: i64, ec: AddressEvent, t: &TransactionWithObservationsAndPrice) -> RgResult<()> {
        let mut amount = CurrencyAmount::from_rdg(0);
        // TODO: Does this need to be a detect on all addresses?
        let party_key_rdg_address = self.key_address().ok_msg("key address")?;
        let incoming = ec.incoming();

        if ec.incoming() {
            amount = t.tx.output_rdg_amount_of_address(&party_key_rdg_address);
            // Is Swap
            let dest = t.tx.swap_destination();
            if let Some(swap_destination) = dest {
                // Represents a withdrawal from network / swap initiation event
                self.fulfill_order(
                    amount.clone(), false, time, None, &swap_destination, false, e, None, swap_destination.currency_or(),
                    e.clone(),
                    None
                )?;
            } else if t.tx.is_stake() {
                self.handle_stake_requests(e, time, &t.tx)?;
                // Represents a stake deposit initiation event OR just a regular transaction sending here
                // TODO: Don't match this an else, but rather allow both swaps and stakes as part of the same TX.
            } else if t.tx.has_portfolio_request() {
                self.handle_portfolio_request(e, time, &t.tx)?;
            }
        } else {
            // this is where problem is
            let outgoing_amount = t.tx.output_rdg_amount_of_exclude_address(&party_key_rdg_address);
            amount = outgoing_amount;
            // This is an outgoing transaction representing a deposit fulfillment receipt
            for tx_id in t.tx.output_external_txids() {
                self.remove_unconfirmed_event(e);
                let mut found_match = false;
                self.unfulfilled_incoming_external_amount_to_outgoing_rdg_orders.retain(|(of, d)| {
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
                    self.unfulfilled_incoming_external_amount_to_outgoing_rdg_orders.retain(|(of, d)| {
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


    fn fulfillment_orders(&self, c: SupportedCurrency) -> Vec<OrderFulfillment> {
        self.orders().iter().filter(|o| o.destination.currency_or() == c).cloned().collect()
    }

    fn orders(&self) -> Vec<OrderFulfillment> {
        let mut orders = vec![];

        let rdg_extern_txids = self.unconfirmed_rdg_output_btc_txid_refs();

        //
        for (of, ae) in self.unfulfilled_incoming_external_amount_to_outgoing_rdg_orders.iter() {
            match ae {
                AddressEvent::External(t) => {

                    // info!("External event id: {} {}", t.tx_id.clone(), t.clone().other.unwrap().render_string().unwrap_or("".to_string()));
                    // Since this is a BTC incoming transaction,
                    // we need to check for unconfirmed events that have the txid in one of the output refs
                    // info!("On match statement for ")
                    if !rdg_extern_txids.contains(&t.tx_id) {
                        orders.push(of.clone());
                    }
                }
                AddressEvent::Internal(_) => {}
            }
        }

        for (of, ae) in &self.unfulfilled_internal_tx_requiring_external_outgoing_mpc_withdrawals {
            // Disable stake withdrawals temporarily on mainnet
            if of.is_stake_withdrawal && self.network.is_main() {
                continue;
            }
            let mpc_claims_fulfillment = self.locally_fulfilled_orders.iter().filter(|f| &f.primary_event == ae)
                .next().is_some();
            match ae {
                AddressEvent::Internal(t) => {
                    // Since this is a RDG incoming transaction, which we'll fulfill with BTC / pair,
                    // We need to know it's corresponding BTC address to see if an unconfirmed output matches it
                    // (i.e. it's already been unconfirmed fulfilled.)
                    //. TODO this is wrong
                    let fulfillment_destination =
                        t.tx.swap_destination().or(t.tx.stake_withdrawal_destination())
                            .and_then(|a| a.render_string().ok());

                    fulfillment_destination.map(|addr| {
                        if !self.unconfirmed_output_other_addresses().contains(&addr) {
                            if mpc_claims_fulfillment {
                                Err::<(), ErrorInfo>(error_info("MPC claims event fulfillment but external TXID not yet recognized as unconfirmed"))
                                    .with_detail("txid", t.tx.hash_hex())
                                    .with_detail("event", ae.json_or())
                                    // .log_error()
                                    .ok();
                            } else {
                                orders.push(of.clone());
                            }
                        }
                    });
                }
                External(_) => {}
            }
        }

        let mut filtered_orders = vec![];

        for o in orders {
            let currency = o.destination.currency_or();
            if let Some(b) = self.balance_map.get(&currency) {
                let fee = Self::expected_fee_amount(currency, &self.network).unwrap_or(CurrencyAmount::zero(currency));
                let total = o.fulfilled_currency_amount() + fee;
                if b < &total {
                    continue;
                }
            }
            filtered_orders.push(o);
        }


        filtered_orders.sort_by(|a, b| a.event_time.cmp(&b.event_time));
        filtered_orders
    }

    fn new(
        network: &NetworkEnvironment,
        relay: &Relay,
        party_addresses: HashMap<SupportedCurrency, Vec<Address>>
    ) -> Self {
        // let btc_rdg = get_btc_per_rdg_starting_min_ask(0);
        // let min_ask = btc_rdg;
        // let price = 1f64 / btc_rdg;
        Self {
            network: network.clone(),
            events: vec![],
            balance_map: Default::default(),
            balance_pending_order_deltas_map: Default::default(),
            balance_with_deltas_applied: Default::default(),
            unfulfilled_incoming_external_amount_to_outgoing_rdg_orders: vec![],
            unfulfilled_internal_tx_requiring_external_outgoing_mpc_withdrawals: vec![],
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
            // relay: Some(relay.clone()),
            central_price_history: Some(vec![]),
            locally_fulfilled_orders: vec![],
            portfolio_request_events: Default::default(),
            default_fee_addrs: relay.default_fee_addrs(),
            seeds: relay.node_config.seeds_now_pk(),
            party_addresses,
        }
    }
}
