use std::collections::{HashMap, HashSet};
use std::ops::{Add, Sub};
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use itertools::Itertools;
use log::{error, info};
use rocket::serde::{Deserialize, Serialize};
use redgold_keys::address_external::ToBitcoinAddress;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::util::btc_wallet::{ExternalTimedTransaction, SingleKeyBitcoinWallet};
use redgold_schema::{EasyJson, error_info, RgResult, structs, WithMetadataHashable};
use redgold_schema::structs::{Address, CurrencyAmount, ErrorInfo, ExternalTransactionId, NetworkEnvironment, ObservationProof, PublicKey, State, SupportedCurrency, Transaction, ValidationLiveness};
use crate::api::public_api::PublicClient;
use crate::api::RgHttpClient;
use crate::core::relay::Relay;
use crate::multiparty::watcher::{BidAsk, DepositWatcher, get_btc_per_rdg_starting_min_ask, OrderFulfillment};
use crate::node_config::NodeConfig;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionWithObservations {
    tx: Transaction,
    observations: Vec<ObservationProof>
}


#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Balance {
    value: i64,
    currency: SupportedCurrency
}

impl Balance {
    pub fn new(value: i64, currency: SupportedCurrency) -> Self {
        Self {
            value,
            currency
        }
    }
    pub fn btc(value: u64) -> Self {
        Self::new(value as i64, SupportedCurrency::Bitcoin)
    }
    pub fn rdg(value: i64) -> Self {
        Self::new(value, SupportedCurrency::Redgold)
    }
    pub fn rdga(value: CurrencyAmount) -> Self {
        Self::new(value.amount, SupportedCurrency::Redgold)
    }

    pub fn add(&mut self, amount: i64) {
        self.value += amount;
    }

    pub fn subtract(&mut self, amount: i64) {
        self.value -= amount;
    }


}


pub struct PartyEvents {
    key_address: Address,
    party_public_key: structs::PublicKey,
    relay: Relay,
    pub events: Vec<AddressEvent>,
    pub balance_map: HashMap<SupportedCurrency, i64>,
    pub unfulfilled_deposits: Vec<(OrderFulfillment, AddressEvent)>,
    pub unfulfilled_withdrawals: Vec<(OrderFulfillment, AddressEvent)>,
    pub price: f64,
    pub(crate) bid_ask: BidAsk,
    pub unconfirmed_events: Vec<AddressEvent>,
    // TODO: populate
    pub fulfillment_history: Vec<(OrderFulfillment, AddressEvent, AddressEvent)>
}

impl PartyEvents {
    //
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

    pub fn orders(&self) -> Vec<OrderFulfillment> {
        let mut orders = vec![];

        let rdg_extern_txids = self.unconfirmed_rdg_output_btc_txid_refs();

        for (of, ae) in self.unfulfilled_deposits.iter() {
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

        for (of, ae) in &self.unfulfilled_withdrawals {
            match ae {
                AddressEvent::Internal(t) => {
                    // Since this is a RDG incoming transaction, which we'll fulfill with BTC,
                    // We need to know it's corresponding BTC address to see if an unconfirmed output matches it
                    // (i.e. it's already been unconfirmed fulfilled.)
                    t.tx.first_input_address_to_btc_address(&self.relay.node_config.network).map(|addr| {
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
    pub fn new(party_public_key: &PublicKey, relay: &Relay) -> Self {
        let btc_rdg = get_btc_per_rdg_starting_min_ask(0);
        let min_ask = btc_rdg;
        let price = 1f64 / btc_rdg;
        Self {
            key_address: party_public_key.address().expect("works").clone(),
            party_public_key: party_public_key.clone(),
            relay: relay.clone(),
            events: vec![],
            balance_map: Default::default(),
            unfulfilled_deposits: vec![],
            unfulfilled_withdrawals: vec![],
            price: price,
            bid_ask: BidAsk::generate_default(
                0, 0, price, min_ask
            ),
            unconfirmed_events: vec![],
            fulfillment_history: vec![],
        }
    }

    pub async fn process_event(&mut self, e: &AddressEvent) -> RgResult<()> {
        let time = e.time(&self.relay.node_config.seeds_pk());
        if let Some(t) = time {
            self.process_confirmed_event(e, t).await?;
        } else {
            self.unconfirmed_events.push(e.clone());
        }

        Ok(())
    }

    async fn process_confirmed_event(&mut self, e: &AddressEvent, time: i64) -> Result<(), ErrorInfo> {
        let ec = e.clone().clone();
        let mut event_fulfillment: Option<OrderFulfillment> = None;
        match e {
            // External Bitcoin Transaction event
            AddressEvent::External(t) => {
                let balance = self.balance_map.get(&t.currency).map(|b| b.clone()).unwrap_or((0i64));
                let mut balance_sign = 1;

                if t.incoming {
                    // Represents a deposit / swap external event.
                    // This should be a fulfillment of an ASK, corresponding to a TAKER BUY
                    // Corresponding to a price increase
                    // Event initiator, has no pairing event yet (short of staking requests)
                    // Balance / price adjustment event

                    // Expect BTC here
                    let other_addr = t.other_address_typed().expect("addr");
                    let fulfillment = self.bid_ask.fulfill_taker_order(
                        t.amount, true, time, Some(t.tx_id.clone()), &other_addr
                    );
                    // info!("Incoming BTC tx {} Fulfillment: {}", t.json_or(), fulfillment.json_or());
                    if let Some(fulfillment) = fulfillment {
                        event_fulfillment = Some(fulfillment.clone());
                        let pair = (fulfillment, ec.clone());
                        self.unfulfilled_deposits.push(pair);
                    }
                } else {
                    balance_sign = -1;
                    // Represents a receipt transaction for outgoing withdrawal.
                    // Should have a paired internal deposit event
                    self.unfulfilled_withdrawals.retain(|(of, d)| {
                        let res = Self::retain_unfulfilled_withdrawals(&t, d, &self.party_public_key, &self.relay.node_config.network);
                        if !res {
                            // This represents and outgoing BTC fulfillment of an incoming RDG tx
                            let fulfillment = (of.clone(), d.clone(), ec.clone());
                            self.fulfillment_history.push(fulfillment);
                            // info!("Outgoing BTC tx fulfillment with hash: {} to {} fulfillment {}", t.tx_id.clone(), t.other_address, of.json_or());
                        };
                        res
                    });
                    self.remove_unconfirmed_event(&e);
                    // info!("Outgoing BTC tx {}", t.json_or());

                }
                let delta = (t.amount as i64) * balance_sign;
                let new_balance = balance + delta;
                self.balance_map.insert(t.currency.clone(), new_balance);
            }
            // Internal Redgold transaction event
            AddressEvent::Internal(t) => {
                let balance = self.balance_map.get(&SupportedCurrency::Redgold).cloned().unwrap_or((0i64));
                let mut balance_sign = -1;
                let mut amount = 0;
                let incoming = !t.tx.input_addresses().contains(&self.key_address);
                if incoming {

                    balance_sign = 1;
                    amount = t.tx.output_amount_of_multi(&self.party_public_key, &self.relay.node_config.network).unwrap_or(0);
                    let is_swap = t.tx.has_swap_to_multi(&self.party_public_key, &self.relay.node_config.network);
                    if is_swap {
                        // Represents a withdrawal initiation event
                        if let Some(addr) = t.tx.first_input_address_to_btc_address(&self.relay.node_config.network) {
                            let addr = Address::from_bitcoin(&addr);
                            let fulfillment = self.bid_ask.fulfill_taker_order(
                                amount as u64, false, time, None, &addr
                            );
                            if let Some(fulfillment) = fulfillment {
                                event_fulfillment = Some(fulfillment.clone());
                                let pair = (fulfillment.clone(), ec.clone());
                                self.unfulfilled_withdrawals.push(pair);
                                // info!("Withdrawal fulfillment request for incoming RDG tx_hash: {} fulfillment {}", t.tx.hash_or(), fulfillment.json_or());
                            }
                        };
                    } else {
                        // Represents a stake deposit initiation event
                    }
                } else {
                    let outgoing_amount = t.tx.non_remainder_amount();
                    amount = outgoing_amount;
                    // This is an outgoing transaction representing a deposit fulfillment receipt
                    for tx_id in t.tx.output_external_txids() {
                        self.remove_unconfirmed_event(e);
                        self.unfulfilled_deposits.retain(|(of, d)| {
                            let res = Self::retain_unfulfilled_deposits(tx_id, d);
                            if !res {
                                let fulfillment = (of.clone(), d.clone(), ec.clone());
                                self.fulfillment_history.push(fulfillment);
                            }
                            res
                        });
                        // info!("Outgoing RDG tx fulfillment for BTC tx_id: {} {}", tx_id.identifier.clone(), t.tx.json_or());
                    }

                }
                let delta = amount as i64 * balance_sign;
                let new_balance = balance + delta;
                self.balance_map.insert(SupportedCurrency::Redgold, new_balance);
            }
        }

        let new_price = if let Some(f) = event_fulfillment {
            let p_delta = f.fulfillment_fraction();
            self.price * (1.0 + p_delta)
        } else {
            self.price
        };
        let min_ask = get_btc_per_rdg_starting_min_ask(time);
        let balance = self.balance_map.get(&SupportedCurrency::Redgold).unwrap_or(&(0i64)).clone() as i64;
        let pair_balance = self.balance_map.get(&SupportedCurrency::Bitcoin).unwrap_or(&(0i64)).clone() as u64;
        self.bid_ask = BidAsk::generate_default(
            balance, pair_balance, new_price, min_ask
        );

        // info!("New bid ask: {}", self.bid_ask.json_or());
        // info!("New balances: {}", self.balance_map.json_or());
        self.price = new_price;
        Ok(())
    }

    fn retain_unfulfilled_deposits(tx_id: &ExternalTransactionId, d: &AddressEvent) -> bool {
        match d {
            AddressEvent::External(t2) => {
                let receipt_match = t2.tx_id == tx_id.identifier;
                !receipt_match
            }
            _ => true
        }
    }

    fn retain_unfulfilled_withdrawals(t: &&ExternalTimedTransaction, d: &AddressEvent, party_public_key: &PublicKey, network: &NetworkEnvironment) -> bool {
        match d {
            AddressEvent::Internal(t2) => {
                let is_swap = t2.tx.has_swap_to_multi(party_public_key, network);
                // RDG transaction previously sent to AMM address with some input address
                // We need to check if the input address is the same as the output address of the current transaction
                // To see if this constitutes a reception outgoing transaction or receipt
                let address_match = t2.tx.input_bitcoin_address(network, &t.other_address);
                let matching_receipt = is_swap && address_match;
                !matching_receipt
            }
            _ => true
        }
    }
    pub async fn historical_initialize(
        pk_address: &PublicKey,
        relay: &Relay,
        btc_wallet: &Arc<Mutex<SingleKeyBitcoinWallet>>,
    ) -> RgResult<Self> {


        let mut n = Self::new(pk_address, relay);
        // transactions

        let seeds = relay.node_config.seeds.iter().flat_map(|s| s.public_key.clone()).collect_vec();

        // First get all transactions associated with the address, both incoming or outgoing.

        let tx = relay.ds.transaction_store
            .get_all_tx_for_address(&n.key_address, 100000, 0).await?;

        let mut res = vec![];
        for t in tx {
            let h = t.hash_or();
            let obs = relay.ds.observation.select_observation_edge(&h).await?;
            let txo = TransactionWithObservations {
                tx: t,
                observations: obs,
            };
            let ae = AddressEvent::Internal(txo);
            res.push(ae);
        }

        btc_wallet.lock().map_err(|e| error_info(format!("Failed to lock wallet: {}", e).as_str()))?
            .get_all_tx()?.iter().for_each(|t| {
            let ae = AddressEvent::External(t.clone());
            res.push(ae);
        });

        res.sort_by(|a, b| a.time(&seeds).cmp(&b.time(&seeds)));

        n.events = res.clone();

        // info!("Watcher Processing {} events", res.len());
        // info!("Watcher Processing events {}", res.json_or());

        for e in &res {
            n.process_event(e).await?;
        }

        Ok(n)

        // let mut staking_deposits = vec![];
        // // TODO: Add withdrawal support
        // // let mut staking_withdrawals = vec![];
        //
        // for t in &tx {
        //     if let Some((amount, liquidity_request)) = t.liquidity_of(&key_address) {
        //         if let Some(d) = &liquidity_request.deposit {
        //             let d = StakeDepositInfo {
        //                 amount: amount.clone(),
        //                 deposit: d.clone(),
        //                 tx_hash: t.hash_or(),
        //             };
        //             staking_deposits.push(d);
        //         }
        //     }
        // }
        //



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
}


#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum AddressEvent {
    External(ExternalTimedTransaction),
    Internal(TransactionWithObservations)
}



impl AddressEvent {

    pub fn identifier(&self) -> String {
        match self {
            AddressEvent::External(e) => e.tx_id.clone(),
            AddressEvent::Internal(t) => t.tx.hash_or().hex()
        }
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
            AddressEvent::External(e) => e.timestamp.map(|t| (t * 1000) as i64),
            AddressEvent::Internal(t) => {
                let seed_obs = t.observations.iter().filter_map(|o|
                    {
                        let metadata = o.proof.as_ref()
                            .and_then(|p| p.public_key.as_ref())
                            .filter(|pk| seeds.contains(pk))
                            .and_then(|pk| o.metadata.as_ref());
                        metadata
                            .and_then(|m| m.time().ok())
                            .filter(|_| metadata.filter(|m| m.validation_liveness == ValidationLiveness::Live as i32).is_some())
                            .filter(|_| metadata.filter(|m| m.state == State::Accepted as i32).is_some())
                    }
                ).map(|t| t.clone()).collect_vec();
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


#[async_trait]
pub trait AllTxObsForAddress {
    async fn get_all_tx_obs_for_address(&self, address: &Address, limit: i64, offset: i64) -> RgResult<Vec<TransactionWithObservations>>;
}

#[async_trait]
impl AllTxObsForAddress for RgHttpClient {
    async fn get_all_tx_obs_for_address(&self, address: &Address, limit: i64, offset: i64) -> RgResult<Vec<TransactionWithObservations>> {

        // self.query_hash(address.render_string())
        // let tx = self.get_all_tx_for_address(address, limit, offset).await?;
        // let mut res = vec![];
        // for t in tx {
        //     let h = t.hash_or();
        //     let obs = self.select_observation_edge(&h).await?;
        //     let txo = TransactionWithObservations {
        //         tx: t,
        //         observations: obs,
        //     };
        //     res.push(txo);
        // }
        // Ok(res)
        Err(error_info("Not implemented"))
    }
}

#[ignore]
#[tokio::test]
async fn debug_event_stream() {
    debug_events().await.unwrap();
}
async fn debug_events() -> RgResult<()> {


    let pk_hex = "03879516077881c5be714024099c16974910d48b691c94c1824fad9635c17f3c37";
    let pk_address = PublicKey::from_hex(pk_hex).expect("pk");

    let relay = Relay::dev_default().await;

    let mut btc_wallet =
    Arc::new(Mutex::new(
        SingleKeyBitcoinWallet::new_wallet(pk_address.clone(), NetworkEnvironment::Dev, true)
            .expect("w")));

    let mut n = PartyEvents::historical_initialize(&pk_address, &relay, &btc_wallet).await?;


    let mut txids = HashSet::new();
    let mut txidsbtc = HashSet::new();

    for e in &n.events {
        match e {
            AddressEvent::External(t) => {
                if t.incoming {
                    txidsbtc.insert(t.tx_id.clone());
                }
            }
            AddressEvent::Internal(int) => {
                if let Some(txid) = int.tx.first_output_external_txid() {
                    txids.insert(txid.identifier.clone());
                }
            }
        }
    };

    let mut missing = txidsbtc.sub(&txids);

    // transactions
    //
    // let seeds = relay.node_config.seeds.iter().flat_map(|s| s.public_key.clone()).collect_vec();
    //
    // // First get all transactions associated with the address, both incoming or outgoing.
    //
    // let txf = relay.ds.transaction_store
    //     .get_filter_tx_for_address(&n.key_address, 10000, 0, true).await?;
    //
    // let tx = relay.ds.transaction_store
    //     .get_all_tx_for_address(&n.key_address, 100000, 0).await?;
    //
    // let mut res = vec![];
    // for t in tx {
    //     let h = t.hash_or();
    //     let obs = relay.ds.observation.select_observation_edge(&h).await?;
    //     let txo = TransactionWithObservations {
    //         tx: t,
    //         observations: obs,
    //     };
    //     let ae = AddressEvent::Internal(txo);
    //     res.push(ae);
    // }

    // btc_wallet.lock().map_err(|e| error_info(format!("Failed to lock wallet: {}", e).as_str()))?
    //     .get_all_tx()?.iter().for_each(|t| {
    //     let ae = AddressEvent::External(t.clone());
    //     res.push(ae);
    // });
    //
    // res.sort_by(|a, b| a.time(&seeds).cmp(&b.time(&seeds)));
    //
    // n.events = res.clone();
    //
    // for e in &res {
    //     n.process_event(e).await?;
    // }


    let orders = n.orders();

    Ok(())

    // DepositWatcher::get_starting_center_price_rdg_btc_fallback()

}
