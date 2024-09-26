// use std::sync::{Arc, Mutex};
// use async_trait::async_trait;
// use bdk::database::MemoryDatabase;
// use futures::TryFutureExt;
// use itertools::Itertools;
// use tracing::{error, info};
//
// use redgold_schema::{error_info, ErrorInfoContext, from_hex, from_hex_ref, RgResult, SafeOption, structs};
// use redgold_schema::structs::{Address, BytesData, CurrencyAmount, ErrorInfo, ExternalTransactionId, Hash, InitiateMultipartyKeygenRequest, StakeDeposit, MultipartyIdentifier, NetworkEnvironment, PartyId, PartyInfo, PublicKey, SubmitTransactionResponse, SupportedCurrency, Transaction, UtxoEntry, Weighting};
// use crate::core::relay::Relay;
// use crate::core::stream_handlers::IntervalFold;
// use crate::multiparty_gg20::initiate_mp;
//
// use serde::{Deserialize, Serialize};
// use redgold_data::data_store::DataStore;
// use redgold_keys::transaction_support::TransactionSupport;
// use crate::core::transact::tx_builder_supports::TransactionBuilder;
// use redgold_keys::util::btc_wallet::{ExternalTimedTransaction, SingleKeyBitcoinWallet};
// use crate::multiparty_gg20::initiate_mp::initiate_mp_keysign;
// use crate::node::Node;
// use redgold_keys::address_external::ToBitcoinAddress;
// use redgold_keys::address_support::AddressSupport;
// use redgold_schema::observability::errors::Loggable;
// use redgold_schema::helpers::easy_json::EasyJson;
// use redgold_schema::helpers::easy_json::EasyJsonDeser;
// use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
// use redgold_schema::observability::errors::EnhanceErrorInfo;
// use crate::core::transact::tx_builder_supports::TransactionBuilderSupport;
// use crate::party::party_stream::PartyEvents;
// use crate::node_config::NodeConfig;
// use crate::party::bid_ask::BidAsk;
// use crate::party::deposit_key_allocation::DepositKeyAllocation;
// use crate::party::price_volume::{PriceVolume, PriceVolumeBroken};
// use crate::scrape::coinbase_btc_spot_latest;
// use crate::util;
// use crate::util::cli::arg_parse_config::ArgTranslate;
// use crate::util::cli::args::RgArgs;
// use crate::util::current_time_millis_i64;
//

//
// // #[test]
// // fn inspect_price_volume() {
// //     let pv = PriceVolume::generate(1_000_000, 1., 25, -0.5, 10.0);
// //     for p in pv.iter() {
// //         println!("{}, {}", p.price, p.volume);
// //     }
// // }
//

//
//
// #[derive(Clone, Serialize, Deserialize)]
// pub struct WithdrawalBitcoin {
//     outputs: Vec<(String, u64)>,
//     updated_bidask: BidAsk,
//     used_tx: Vec<Transaction>
// }
//
// #[derive(Serialize, Deserialize, Clone)]
// pub struct DepositWatcherConfig {
//     pub deposit_allocations: Vec<DepositKeyAllocation>,
//     // TODO: Make this a map over currency type
//     pub bid_ask: BidAsk,
//     pub last_btc_timestamp: u64,
//     pub ask_bid_code_reset: Option<bool>,
// }
//
// #[derive(Serialize, Deserialize, Clone)]
// pub struct BidAskBroken{
//     pub bids: Vec<PriceVolumeBroken>,
//     pub asks: Vec<PriceVolumeBroken>,
//     pub center_price: f64
// }
//
//
// #[derive(Serialize, Deserialize, Clone)]
// pub struct DepositWatcherConfigBroken {
//     pub deposit_allocations: Vec<DepositKeyAllocation>,
//     // TODO: Make this a map over currency type
//     pub bid_ask: BidAskBroken,
//     pub last_btc_timestamp: u64,
//     pub ask_bid_code_reset: Option<bool>
// }
//
// #[derive(Clone)]
// pub struct DepositWatcher {
//     relay: Relay,
//     wallet: Vec<Arc<Mutex<SingleKeyBitcoinWallet<MemoryDatabase>>>>
// }
//
// impl DepositWatcher {
//
//     // Need to update this to the current test address?
//     pub async fn genesis_funding(&self, destination: &Address) -> RgResult<()> {
//         let (_, utxos) = Node::genesis_from(self.relay.node_config.clone());
//         let u = utxos.get(14).safe_get_msg("Missing utxo")?.clone();
//         let a = u.key_pair.address_typed();
//         let a_str = a.render_string()?;
//         let utxo_id = &u.utxo_entry.utxo_id()?.clone();
//         let res = self.relay.ds.utxo.utxo_id_valid(
//             utxo_id
//         ).await?;
//         let uu = u.utxo_entry.clone().json_or();
//         if res {
//             info!("Sending genesis funding to multiparty address from origin {a_str} using utxo {uu}");
//             let mut tb = TransactionBuilder::new(&self.relay.node_config);
//             tb.with_utxo(&u.utxo_entry)?;
//             tb.with_output(&destination, &CurrencyAmount::from(u.utxo_entry.amount() as i64));
//             tb.with_stake_usd_bounds(Some(100f64), Some(1000f64), &a);
//             let mut tx = tb.build()?;
//             tx.sign(&u.key_pair)?;
//             self.relay.submit_transaction_sync(&tx).await?;
//         } else {
//             info!("No genesis funding possible to send");
//         }
//         Ok(())
//     }
//
//     pub async fn get_deposit_config(ds: &DataStore) -> Result<Option<DepositWatcherConfig>, ErrorInfo> {
//         ds.config_store.get_json::<DepositWatcherConfig>("deposit_watcher_config").await
//     }
// }
//
// #[derive(Clone, Serialize, Deserialize)]
// pub struct CurveUpdateResult {
//     updated_bid_ask: BidAsk,
//     updated_btc_timestamp: u64,
//     updated_allocation: DepositKeyAllocation
// }
// #[derive(Clone, Serialize, Deserialize)]
// pub struct StakeDepositInfo {
//     amount: CurrencyAmount,
//     deposit: StakeDeposit,
//     tx_hash: Hash
// }
//
// // 100 / 45000
// const BTC_RDG_STARTING: f64 = 0.00222222222f64;
//
//
// // Needs to be hard-coded to deal with event stream changes.
// pub fn get_btc_per_rdg_starting_min_ask(time: i64) -> f64 {
//     return if time > 1707715165171i64 {
//         BTC_RDG_STARTING
//     } else {
//         BTC_RDG_STARTING
//     }
// }
//
// impl DepositWatcher {
//     pub fn new(relay: Relay) -> Self {
//         Self {
//             relay,
//             wallet: vec![],
//         }
//     }
//     // pub fn establish_first_allocation(&self) -> RgResult<()> {
//     //
//     // }
//
//     // TODO: From oracle or api
//     pub async fn convert_btc_amount_usd(_timestamp: u64, _amount: u64) -> f64 {
//         0.
//     }
//
//     // pub async fn get_btc_deposits(&mut self, last_timestamp: u64, w: &Arc<Mutex<SingleKeyBitcoinWallet>>) -> Result<(u64, Vec<ExternalTimedTransaction>), ErrorInfo>{
//     //     let pk_hex = w.lock()
//     //         .map_err(|e| error_info(format!("Failed to lock wallet: {}", e).as_str()))?
//     //         .public_key.hex_or();
//     //
//     //     let mut sourced_tx = w.lock()
//     //         .map_err(|e| error_info(format!("Failed to lock wallet: {}", e).as_str()))?
//     //         .get_sourced_tx()?;
//     //
//     //     info!("public key: {} Got {} sourced tx raw: {}", pk_hex, sourced_tx.len(), sourced_tx.json_or());
//     //
//     //     let mut max_ts: u64 = last_timestamp;
//     //     sourced_tx.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
//     //     let mut res = vec![];
//     //     for tx in sourced_tx.iter() {
//     //         if let Some(ts) = tx.timestamp {
//     //             if ts <= last_timestamp {
//     //                 continue;
//     //             }
//     //             if ts > max_ts {
//     //                 max_ts = ts;
//     //             }
//     //             res.push(tx.clone());
//     //         } else {
//     //             continue
//     //         }
//     //         let used = self.relay.ds.multiparty_store.check_bridge_txid_used(&from_hex(tx.tx_id.clone())?).await?;
//     //         if used {
//     //             continue
//     //         }
//     //         res.push(tx.clone())
//     //     }
//     //     Ok((max_ts, res.clone()))
//     // }
//     //
//     // pub async fn build_rdg_ask_swap_tx(utxos: Vec<UtxoEntry>,
//     //                                    btc_deposits: Vec<ExternalTimedTransaction>,
//     //                                    bid_ask: BidAsk,
//     //                                    _key_address: &structs::Address,
//     //     min_ask: f64
//     // )
//     //     -> RgResult<(Option<Transaction>, BidAsk)> {
//     //
//     //     let mut bid_ask_latest = bid_ask.clone();
//     //
//     //     // We're building a transaction FROM some stored input balance we have
//     //     // for our pubkey multisig address
//     //     let mut tb = TransactionBuilder::new();
//     //     for u in &utxos {
//     //         // Check contract type here
//     //         // let o = u.output.safe_get_msg("Missing output on UTXO")?;
//     //         // if let Some(o) = &o.contract.as_ref().and_then(|c| c.standard_contract_type) {
//     //         //     if o == StandardContractType::Swap as i32
//     //         // }
//     //         tb.with_maybe_currency_utxo(u)?;
//     //     }
//     //
//     //     for tx in btc_deposits.iter() {
//     //         let destination = tx.other_address.clone();
//     //         let destination_address = structs::Address::from_bitcoin(&destination);
//     //         if let Some(timestamp) = tx.timestamp {
//     //             if let Some(ask_fulfillment) = bid_ask_latest.fulfill_taker_order(
//     //                 tx.amount, true, timestamp as i64, Some(tx.tx_id.clone()), &destination_address
//     //             ) {
//     //                 let destination_amount = ask_fulfillment.fulfilled_amount;
//     //
//     //                 tb.with_output(&destination_address,
//     //                                &CurrencyAmount::from(destination_amount as i64)
//     //                 );
//     //                 tb.with_last_output_deposit_swap_fulfillment(tx.tx_id.clone());
//     //
//     //                 let price = ask_fulfillment.fulfillment_price() * 1.01;
//     //                 bid_ask_latest = bid_ask_latest.regenerate(price, min_ask)
//     //             }
//     //         }
//     //
//     //     }
//     //     let mut tx_ret = None;
//     //     if !btc_deposits.is_empty() {
//     //         tx_ret = Some(tb.build()?);
//     //     }
//     //     Ok((tx_ret, bid_ask_latest))
//     // }
//
//     pub async fn send_ask_fulfillment_transaction(&self, tx: &mut Transaction, identifier: MultipartyIdentifier) -> RgResult<SubmitTransactionResponse> {
//
//         let hash = tx.signable_hash();
//         let result = initiate_mp_keysign(self.relay.clone(), identifier.clone(),
//                                          hash.bytes.safe_get()?.clone(), identifier.party_keys.clone(), None,
//
//         ).await?;
//         tx.add_proof_per_input(&result.proof);
//         self.relay.submit_transaction_sync(tx).await
//     }
//
//     pub async fn fulfill_btc_bids(&self, w_arc: &Arc<Mutex<SingleKeyBitcoinWallet<MemoryDatabase>>>,
//                                   identifier: MultipartyIdentifier, outputs: Vec<(String, u64)>) -> RgResult<String> {
//         w_arc.lock()
//             .map_err(|e| error_info(format!("Failed to lock wallet: {}", e).as_str()))?
//             .create_transaction_output_batch(outputs)?;
//         let hashes = w_arc.lock()
//             .map_err(|e| error_info(format!("Failed to lock wallet: {}", e).as_str()))?
//             .signable_hashes()?.clone();
//         for (i, (hash, hash_type)) in hashes.iter().enumerate() {
//             let result = initiate_mp_keysign(self.relay.clone(), identifier.clone(),
//                                              BytesData::from(hash.clone()),
//                                              identifier.party_keys.clone(), None
//             ).await?;
//             w_arc.lock()
//                 .map_err(|e| error_info(format!("Failed to lock wallet: {}", e).as_str()))?
//                 .affix_input_signature(i, &result.proof, hash_type);
//         }
//         let mut w = w_arc.lock()
//             .map_err(|e| error_info(format!("Failed to lock wallet: {}", e).as_str()))?;
//         w.sign()?;
//         w.broadcast_tx()?;
//         Ok(w.txid()?)
//     }
//
//     pub async fn process_requests_new(
//         &mut self,
//         alloc: &DepositKeyAllocation,
//         _bid_ask_original: BidAsk,
//         last_timestamp: u64,
//         w: &Arc<Mutex<SingleKeyBitcoinWallet<MemoryDatabase>>>,
//     ) -> Result<CurveUpdateResult, ErrorInfo> {
//
//         let key = &alloc.key;
//         let key_address = key.address()?;
//
//         let btc_starting_balance = w.lock()
//             .map_err(|e| error_info(format!("Failed to lock wallet: {}", e).as_str()))?
//             .get_wallet_balance()?.confirmed;
//
//         let ps = PartyEvents::historical_initialize(&key, &self.relay, w).await?;
//         let orders = ps.orders();
//         let cutoff_time = current_time_millis_i64() - 30_000; //
//         let identifier = alloc.initiate.identifier.safe_get().cloned()?;
//
//         let environment = self.relay.node_config.network.clone();
//         let btc_address = w.lock()
//             .map_err(|e| error_info(format!("Failed to lock wallet: {}", e).as_str()))?
//             .public_key.to_bitcoin_address(&environment)?;
//
//         let balance = self.relay.ds.transaction_store.get_balance(&key_address).await?;
//         let rdg_starting_balance: i64 = balance.safe_get_msg("Missing balance")?.clone();
//
//
//         let num_events = ps.events.len();
//         let num_unconfirmed = ps.unconfirmed_events.len();
//         let num_unfulfilled_deposits = ps.unfulfilled_deposits.len();
//         let num_unfulfilled_withdrawals = ps.unfulfilled_withdrawals.len();
//         let utxos = self.relay.ds.transaction_store.query_utxo_address(&key_address).await?;
//
//         info!("watcher balances: RDG:{}, BTC:{} \
//          BTC_address: {} environment: {} orders {} num_events: {} num_unconfirmed {} num_un_deposit {} \
//          num_un_withdrawls {} num_utxos: {} orders_json {} \
//          bid_ask: {}",
//             rdg_starting_balance, btc_starting_balance, btc_address, environment.to_std_string(),
//             orders.len(),
//             num_events, num_unconfirmed, num_unfulfilled_deposits, num_unfulfilled_withdrawals,
//             utxos.len(),
//             orders.json_or(),
//             ps.central_prices.json_or()
//         );
//
//
//         // let (asks, bids) = orders.iter()
//         //     .filter(|e| e.event_time < cutoff_time)
//         //     .partition(|o| o.is_ask).collect::<(Vec<OrderFulfillment>, Vec<OrderFulfillment>)>();
//         // TODO: Change this to support batches -- might need some consideration around ids and utxos later
//         // when calculating the receipts?
//
//         let with_cutoff = orders.iter()
//             .filter(|o| o.event_time < cutoff_time)
//             .collect_vec();
//
//         /*
//
//         tb.with_utxos(&utxos)?;
//         let destination_amount = self.fulfilled_currency_amount();
//         tb.with_output(&self.destination, &destination_amount);
//         let option = self.tx_id_ref.safe_get_msg("Missing tx_id")?.clone();
//         tb.with_last_output_deposit_swap(option.identifier);
//         tb.build()
//          */
//         let mut tb = TransactionBuilder::new(&self.relay.node_config);
//         tb.with_utxos(&utxos)?;
//
//         let rdg_fulfillment_txb = with_cutoff.iter()
//             .filter(|e| e.is_ask_fulfillment_from_external_deposit && e.tx_id_ref.is_some())
//             .fold(&mut tb, |tb, o| {
//             tb.with_output(&o.destination, &o.fulfilled_currency_amount())
//             .with_last_output_deposit_swap_fulfillment(o.tx_id_ref.clone().expect("Missing tx_id").identifier, SupportedCurrency::Bitcoin).expect("works")
//         });
//
//         if rdg_fulfillment_txb.transaction.outputs.len() > 0 {
//             let tx = rdg_fulfillment_txb.build()?;
//             info!("Sending RDG fulfillment transaction: {}", tx.json_or());
//             self.send_ask_fulfillment_transaction(&mut tx.clone(), identifier.clone()).await.log_error().ok();
//         }
//
//         let mut outputs = vec![];
//         let btc_outputs = with_cutoff.iter()
//             .filter(|e| !e.is_ask_fulfillment_from_external_deposit &&
//             e.destination.to_bitcoin_address(&self.relay.node_config.network).is_ok())
//             .fold(&mut outputs, |vec, o| {
//                 let btc = o.destination.to_bitcoin_address(&self.relay.node_config.network).expect("works");
//                 let amount = o.fulfilled_amount;
//                 let outputs = (btc, amount);
//                 vec.push(outputs);
//                 vec
//             });
//
//         if btc_outputs.len() > 0 {
//             let txid = self.fulfill_btc_bids(w, identifier.clone(), btc_outputs.clone()).await.log_error().ok();
//             info!("Sending BTC fulfillment transaction id {}: {:?}", txid.json_or(), btc_outputs);
//         }
//         let mut alloc2 = alloc.clone();
//         alloc2.balance_btc = btc_starting_balance;
//         alloc2.balance_rdg = rdg_starting_balance as u64;
//
//         let cur = CurveUpdateResult {
//             updated_bid_ask: _bid_ask_original.clone(),
//             updated_btc_timestamp: last_timestamp,
//             updated_allocation: alloc2.clone()
//         };
//
//         // asks.iter().chunks(10).for_each(|chunk| {
//         //     let mut txs = vec![];
//         //     let mut bid_ask_latest = bid_ask_original.clone();
//         //     for ask in chunk {
//         //         let mut tx = ask.build_rdg_ask_swap_tx(ps.utxos.clone());
//         //         if let Some(tx) = tx {
//         //             self.send_ask_fulfillment_transaction(&mut tx.clone(), alloc.initiate.identifier.clone()).await?;
//         //         }
//         //         let updated_bid_ask = bid_ask_latest.clone();
//         //         bid_ask_latest = ask.updated_curve.clone();
//         //         txs.push(tx);
//         //     }
//         // });
//
//         Ok(cur)
//     }
//
//     // Returns price in RDG/BTC, i.e. ~300 for USD/RDG 100 and BTC 30k
//     pub async fn get_starting_center_price_rdg_btc() -> RgResult<f64> {
//         let usd_btc = coinbase_btc_spot_latest().await?.usd_btc()?;
//         let starting_usd = 100.0;
//         let rdg_btc = usd_btc / starting_usd;
//         Ok(rdg_btc)
//     }
//
//     pub async fn get_starting_center_price_rdg_btc_fallback() -> f64 {
//
//         Self::get_starting_center_price_rdg_btc().await
//             .add("Failed getting BTC/USD spot").log_error()
//             .unwrap_or(BTC_RDG_STARTING) // 100 / 39000
//
//     }
//
//     pub async fn fix_historical_errors(&self) -> RgResult<()> {
//         let ds = self.relay.ds.clone();
//
//         let test_load = ds.config_store.get_json::<DepositWatcherConfig>("deposit_watcher_config").await;
//
//         // First broken json error
//         if test_load.is_err() {
//             let broken_cfg = ds.config_store.get_json::<DepositWatcherConfigBroken>("deposit_watcher_config").await;
//             if let Ok(Some(bcfg)) = broken_cfg {
//                 let ba = bcfg.bid_ask;
//                 let new_bid_ask = BidAsk {
//                     bids: ba.bids.iter().filter_map(|v| {
//                         if let Some(p) = v.price {
//                             if let Some(v) = v.volume {
//                                 Some(PriceVolume { price: p, volume: v })
//                             } else {
//                                 None
//                             }
//                         } else {
//                             None
//                         }
//                     }).collect::<Vec<PriceVolume>>(),
//                     asks: ba.asks.iter().filter_map(|v| {
//                         if let Some(p) = v.price {
//                             if let Some(v) = v.volume {
//                                 Some(PriceVolume { price: p, volume: v })
//                             } else {
//                                 None
//                             }
//                         } else {
//                             None
//                         }
//                     }).collect::<Vec<PriceVolume>>(),
//                     center_price: Self::get_starting_center_price_rdg_btc_fallback().await,
//                 };
//                 let new_cfg = DepositWatcherConfig {
//                     deposit_allocations: bcfg.deposit_allocations,
//                     bid_ask: new_bid_ask,
//                     last_btc_timestamp: 0,
//                     ask_bid_code_reset: None,
//                 };
//                 ds.config_store.insert_update_json("deposit_watcher_config", new_cfg).await?;
//                 info!("Updated broken deposit watcher config");
//             };
//         }
//         Ok(())
//     }
//
// }
//
//
// #[async_trait]
// impl IntervalFold for DepositWatcher {
//
//     #[tracing::instrument(skip(self))]
//     async fn interval_fold(&mut self) -> RgResult<()> {
//
//         // info!("Deposit watcher interval fold complete");
//
//         if self.relay.node_config.is_local_debug() {
//             return Ok(())
//         }
//
//         let ds = self.relay.ds.clone();
//         // TODO: Change to query to include trust information re: deposit score
//         // How best to represent this to user? As trustData?
//         let _nodes = ds.peer_store.active_nodes(None).await?;
//         self.fix_historical_errors().await.log_error().ok();
//
//         // Fund from genesis for test purposes
//         // self.genesis_funding().await?;
//
//         // let kp = initiate_mp::find_multiparty_key_pairs(self.relay.clone()).await;
//         // match kp {
//         //     Ok(_) => {}
//         //     Err(_) => {}
//         // }
//
//
//         let cfg = Self::get_deposit_config(&ds).await?;
//
//         //.ok.andthen?
//         if let Some(mut cfg) = cfg {
//             // if cfg.ask_bid_code_reset.is_none() {
//             //     info!("Regenerating starting price due to code reset");
//             //     cfg.bid_ask = cfg.bid_ask.regenerate(self.get_starting_center_price_rdg_btc().await);
//             //     cfg.ask_bid_code_reset = Some(true);
//             //     ds.config_store.insert_update_json("deposit_watcher_config", cfg.clone()).await?;
//             // }
//
//             // Check to see if other nodes are dead / not responding, if so, move the thing.
//             // Also check bitcoin transaction balances? Find the address they came from.
//             // we'll need a guide saying to send from a single account
//             if let Some(d) = cfg.deposit_allocations.get(0) {
//                 self.relay.add_party_id(&d.party_id()?).await?;
//                 // info!("Watcher checking deposit allocation pubkey hex: {}", d.key.hex()?);
//                 if self.wallet.get(0).is_none() {
//                     let key = &d.key;
//                     let w = SingleKeyBitcoinWallet::new_wallet(key.clone(), self.relay.node_config.network, true)?;
//                     self.wallet.push(Arc::new(Mutex::new(w)));
//                 }
//                 let w = self.wallet.get(0).cloned();
//                 if let Some(w) = w {
//                     let btc_starting_balance = w.lock()
//                         .map_err(|e| error_info(format!("Failed to lock wallet: {}", e).as_str()))?
//                         .get_wallet_balance()?.confirmed;
//
//                     let balance = self.relay.ds.transaction_store.get_balance(&d.key.address()?).await?;
//                     if balance.map(|x| x > 0).unwrap_or(false) { // && btc_starting_balance > 3500 {
//                         let reset_condition = true;
//                         if cfg.ask_bid_code_reset == Some(reset_condition) {
//                             info!("Regenerating starting price due to code reset");
//                             let center_price = DepositWatcher::get_starting_center_price_rdg_btc_fallback().await;
//                             let min_ask = 1f64 / center_price;
//                             cfg.bid_ask = cfg.bid_ask.regenerate(center_price, min_ask);
//                             cfg.ask_bid_code_reset = Some(!reset_condition);
//                             ds.config_store.insert_update_json("deposit_watcher_config", cfg.clone()).await?;
//                         }
//                         let update_result = self.process_requests_new(
//                             d, cfg.bid_ask.clone(), cfg.last_btc_timestamp, &w
//                         ).await;
//                         if let Ok(update_result) = &update_result {
//                             let mut cfg2 = cfg.clone();
//                             cfg2.last_btc_timestamp = update_result.updated_btc_timestamp;
//                             cfg2.bid_ask = update_result.updated_bid_ask.clone();
//                             cfg2.deposit_allocations = vec![update_result.updated_allocation.clone()];
//                             ds.config_store.insert_update_json("deposit_watcher_config", cfg2).await?;
//                         } else if let Err(e) = update_result {
//                             error!("Error processing requests: {}", e.json_or());
//                         }
//                     } else {
//                         info!("No balance found for key: {} or insufficient bitcoin balance of {}", d.key.address()?.render_string()?, btc_starting_balance);
//                     }
//                 }
//             }
//         } else {
//             info!("Attempting to start MP watcher keygen round");
//             // Initiate MP keysign etc. gather public key and original proof and params
//             let seeds = self.relay.node_config.non_self_seeds().clone();
//             let min_seeds = if self.relay.node_config.network.is_local_debug() {
//                 2
//             } else {
//                 3
//             };
//
//             if seeds.len() <= min_seeds {
//                 error!("Not enough seeds to initiate MP keygen {} {}", seeds.len(), seeds.json_or());
//                 return Ok(())
//             }
//
//             let pks = seeds.iter().flat_map(|s| s.public_key.clone()).collect_vec();
//
//             let res = initiate_mp::initiate_mp_keygen(
//                 self.relay.clone(),
//                 None,
//                 true,
//                 Some(pks)
//             ).await.log_error();
//             // TODO: Get this from local share instead of from a second keysign round.
//             if let Ok(r) = res {
//                 let test_sign = r.identifier.room_id.safe_get()?.uuid.safe_get()?.clone();
//                 let h = Hash::from_string_calculate(&test_sign);
//                 let bd = h.bytes.safe_get_msg("Missing bytes in immediate hash calculation")?;
//                 let ksr = initiate_mp::initiate_mp_keysign(
//                     self.relay.clone(), r.identifier.clone(),
//                     bd.clone(),
//                     r.identifier.party_keys.clone(),
//                     None
//                 ).await.log_error();
//                 if let Ok(ksr) = ksr {
//                     // TODO: if not successful, attempt some retries and then delete the operation
//                     // and begin again from keygen.
//                     // or just delete it immediately.
//                     let pk = ksr.proof.public_key.safe_get_msg("Missing public key on key sign result")?;
//                     let cfg = DepositWatcherConfig {
//                         deposit_allocations: vec![DepositKeyAllocation{
//                             key: pk.clone(),
//                             allocation: 1.0,
//                             initiate: r.request.clone(),
//                             balance_btc: 0,
//                             balance_rdg: 0,
//                         }],
//                         bid_ask: BidAsk { bids: vec![], asks: vec![], center_price: Self::get_starting_center_price_rdg_btc_fallback().await },
//                         last_btc_timestamp: 0,
//                         ask_bid_code_reset: None,
//                     };
//                     self.genesis_funding(&pk.address()?)
//                         .await.add("Genesis watcher funding error").log_error().ok();
//                     ds.config_store.insert_update_json("deposit_watcher_config", cfg).await?;
//                 }
//             }
//             // self.relay.broadcast_async(nodes, req)
//         }
//
//         Ok(())
//     }
// }
//
// #[ignore]
// // #[tokio::test]
// async fn debug_local_ds_utxo_balance() {
//     let mut opts = RgArgs::default();
//     opts.network = Some("dev".to_string());
//     let node_config = NodeConfig::default();
//     let mut arg_translate = ArgTranslate::new(&opts, &node_config.clone());
//     arg_translate.translate_args().await.unwrap();
//     let nc = arg_translate.node_config;
//     let r = Relay::new(nc.clone()).await;
//     let a = "cf4989701946ae307efdb902efd73c13d933efda0ef04bcbc3eef2146850534a".parse_address().expect("works");
//     let utxos = r.ds.transaction_store.query_utxo_address(&a).await.unwrap();
//     println!("UTXOS: {}", utxos.json_or());
//     println!("{}", nc.mnemonic_words.clone());
//     let (tx, _gutxos) = Node::genesis_from(nc.clone());
//     // let res = r.ds.transaction_store.query_utxo_output_index(&tx.hash_or()).await.unwrap();
//     // println!("UTXO: {}", res.json_or());
//     println!("Genesis hash {}", tx.hash_or().hex());
//
//
//     //
//     // // Node::prelim_setup(r);
//     // for (i,utxo) in gutxos.iter().enumerate() {
//     //     let res = r.ds.transaction_store.query_utxo_id_valid(&tx.hash_or(), i as i64).await.unwrap();
//     //     if res {
//     //         println!("UTXO {i}: {}", utxo.utxo_entry.json_or());
//     //     }
//     // }
// }
//
//
// #[derive(Serialize, Deserialize)]
// struct TestJson {
//     some: String
// }
//
// #[derive(Serialize, Deserialize)]
// struct TestJson2 {
//     some: String,
//     other: Option<String>
// }
//
// #[test]
// fn test_json() {
//     let t = TestJson{
//         some: "yo".to_string(),
//     };
//     let ser = t.json().expect("works");
//     let t2 = ser.json_from::<TestJson2>().expect("works");
//     assert_eq!(t2.some, "yo".to_string());
//     assert_eq!(t2.other, None);
// }
// //
// // #[ignore]
// // #[tokio::test]
// // async fn debug_local() {
// //     let center_price = DepositWatcher::get_starting_center_price_rdg_btc_fallback().await;
// //     println!("center price: {center_price}");
// //     let min_ask = 1f64 / center_price;
// //     let nc = NodeConfig::dev_default().await;
// //     let c = nc.api_client();
// //     let dev_amm_address = "tb1qyxzxhpdkfdd9f2tpaxehq7hc4522f343tzgvt2".to_string();
// //     let pk_hex = "03879516077881c5be714024099c16974910d48b691c94c1824fad9635c17f3c37";
// //     let pk = PublicKey::from_hex(pk_hex).expect("pk");
// //     let pk_rdg_address = pk.clone().address().expect("address");
// //     let addr = pk_rdg_address.render_string().expect("");
// //     println!("address: {addr}");
// //
// //
// //     let utxos = c.address_info(pk_rdg_address.clone()).await.expect("info").utxo_entries;
// //
// //     let mut w =
// //         SingleKeyBitcoinWallet::new_wallet(pk.clone(), NetworkEnvironment::Dev, true).expect("w");
// //     let a = w.address().expect("a");
// //     println!("wallet address: {a}");
// //     assert_eq!(dev_amm_address, a);
// //     let b = w.get_wallet_balance().expect("balance");
// //     println!("wallet balance: {b}");
// //     let confirmed = b.confirmed;
// //
// //     let rdg_b = (c.balance(pk_rdg_address.clone()).await.expect("balance") * 1e8) as i64;
// //
// //     println!("rdg balance: {rdg_b}");
// //     println!("confirmed: {confirmed}");
// //     println!("center price: {center_price}");
// //
// //
// //     let mut deposit_txs = w.get_sourced_tx().expect("works");
// //     deposit_txs.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
// //
// //     let example_deposit_tx = deposit_txs.last().expect("works");
// //
// //     let ba = BidAsk::generate_default(
// //         rdg_b,
// //         confirmed,
// //         center_price,
// //         min_ask
// //     );
// //
// //     let p = ba.bids.json_pretty_or();
// //
// //     let first_ask = ba.asks.first().expect("works");
// //     let first_ask_price = 1f64/first_ask.price;
// //     println!("first_ask_price: {first_ask_price}");
// //
// //
// //     let first_bid = ba.bids.first().expect("works");
// //     let first_bid_price = first_bid.price;
// //     println!(  "first_bid_price: {first_bid_price}");
// //     //
// //     //
// //     // // println!("bids: {p}");
// //     // let ask_fulfillment = ba.fulfill_taker_order(3500, true, util::current_time_millis_i64(), None).expect("works");
// //     // let afj = ask_fulfillment.fulfilled_amount.json_pretty_or();
// //     //
// //     //
// //     // println!("fullfilled: {afj}");
// //     // println!("fulfilled price: {}", ask_fulfillment.fulfillment_price());
// //     //
// //     // let bid_fulfillment = ba.fulfill_taker_order((2000f64*center_price) as u64, false, util::current_time_millis_i64(), None).expect("works");
// //     //
// //     // let bfj = bid_fulfillment.fulfilled_amount.json_pretty_or();
// //     // println!("fullfilled: {bfj}");
// //     // println!("fulfilled price: {}", 1f64/bid_fulfillment.fulfillment_price());
// //
// //     //
// //     // let (tx, bid_ask_updated_ask_side) = DepositWatcher::build_rdg_ask_swap_tx(
// //     //     utxos,
// //     //     deposit_txs,
// //     //     ba, &pk_rdg_address.clone(), min_ask).await.expect("Workx");
// //     //
// //     // // w.create_transaction(Some(pk), None, 2000).expect("tx");
// //     //
// //     // // let ba2 = ba.regenerate(center_price*1.1f64, min_ask).json_pretty_or();
// //     //
// //     // let txs = tx.json_pretty_or();
// //     // println!("tx: {txs}");
// //
// // }
//
// #[test]
// fn empty() {
//
// }