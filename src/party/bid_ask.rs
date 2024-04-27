// use redgold_schema::structs::{Address, ExternalTransactionId, SupportedCurrency};
// use rocket::serde::{Deserialize, Serialize};
// use crate::party::order_fulfillment::OrderFulfillment;
//
// use crate::party::price_volume::PriceVolume;
// pub const DUST_LIMIT: u64 = 2500;
// #[derive(Serialize, Deserialize, Clone)]
// pub struct BidAsk{
//     pub bids: Vec<PriceVolume>,
//     pub asks: Vec<PriceVolume>,
//     pub center_price: f64
// }
//
// impl BidAsk {
//
//     pub fn asking_price(&self) -> f64 {
//         self.asks.get(0).map(|v| v.price).unwrap_or(0.)
//     }
//
//     pub fn sum_bid_volume(&self) -> u64 {
//         self.bids.iter().map(|v| v.volume).sum::<u64>()
//     }
//
//     pub fn sum_ask_volume(&self) -> u64 {
//         self.asks.iter().map(|v| v.volume).sum::<u64>()
//     }
//
//     pub fn volume_empty(&self) -> bool {
//         self.bids.iter().find(|v| v.volume == 0).is_some() ||
//         self.asks.iter().find(|v| v.volume == 0).is_some()
//     }
//
//     pub fn regenerate(&self, price: f64, min_ask: f64) -> BidAsk {
//         BidAsk::generate_default(
//             self.sum_ask_volume() as i64,
//             self.sum_bid_volume(),
//             price,
//             min_ask
//         )
//     }
//
//     pub fn generate_default(
//         available_balance: i64,
//         pair_balance: u64,
//         last_exchange_price: f64,
//         min_ask: f64,
//     ) -> BidAsk {
//         BidAsk::generate(
//             available_balance,
//             pair_balance,
//             last_exchange_price,
//             40,
//             20.0f64,
//             min_ask
//         )
//     }
//
//     pub fn generate(
//         available_balance_rdg: i64,
//         pair_balance_btc: u64,
//         last_exchange_price: f64, // this is for available type / pair type
//         divisions: i32,
//         scale: f64,
//         // BTC / RDG
//         min_ask: f64
//     ) -> BidAsk {
//
//         // A bid is an offer to buy RDG with BTC
//         // The volume should be denominated in BTC because this is how much is staked natively
//         let bids = if pair_balance_btc > 0 {
//             PriceVolume::generate(
//                 pair_balance_btc,
//                 last_exchange_price, // Price here is RDG/BTC
//                 divisions,
//                 last_exchange_price*0.9,
//                 scale / 2.0
//             )
//         } else {
//             vec![]
//         };
//
//
//         // An ask price in the inverse of a bid price, since we want to denominate in RDG
//         // since the volume is in RDG.
//         // Here it is now BTC / RDG
//         let ask_price_expected = 1.0 / last_exchange_price;
//
//         // Apply a max to ask price.
//         let ask_price = f64::max(ask_price_expected, min_ask);
//
//         // An ask is how much BTC is being asked for each RDG
//         // Volume is denominated in RDG because this is what the contract is holding for resale
//         let asks = if available_balance_rdg > 0 {
//             PriceVolume::generate(
//                 available_balance_rdg as u64,
//                 ask_price,
//                 divisions,
//                 ask_price*3.0,
//                 scale
//             )
//         } else {
//             vec![]
//         };
//         BidAsk {
//             bids,
//             asks,
//             center_price: last_exchange_price,
//         }
//     }
// }
//
// //
// // impl BidAsk {
// //
// //     pub fn remove_empty(&mut self) {
// //         self.bids.retain(|v| v.volume > 0);
// //         self.asks.retain(|v| v.volume > 0);
// //     }
// //     pub fn fulfill_taker_order(
// //         &self,
// //         order_amount: u64,
// //         is_ask: bool,
// //         event_time: i64,
// //         tx_id: Option<String>,
// //         destination: &Address
// //     ) -> Option<OrderFulfillment> {
// //         let mut remaining_order_amount = order_amount.clone();
// //         let mut fulfilled_amount: u64 = 0;
// //         let mut updated_curve = if is_ask {
// //             // Asks are ordered in increasing amount(USD), denominated in BTC/RDG
// //             self.asks.clone()
// //         } else {
// //             // Bids are ordered in decreasing amount(USD), denominated in RDG/BTC
// //             self.bids.clone()
// //         };
// //
// //
// //         for pv in updated_curve.iter_mut() {
// //
// //             let other_amount_requested = if is_ask {
// //                 // Comments left here for clarity even if code is the same
// //                 let price = pv.price; // BTC / RDG
// //                 // BTC / (BTC / RDG) = RDG
// //                 remaining_order_amount as f64 / price
// //             } else {
// //                 // RDG / RDG/BTC = BTC
// //                 remaining_order_amount as f64 / pv.price
// //             } as u64;
// //
// //             let this_vol = pv.volume;
// //             if other_amount_requested >= this_vol {
// //                 // We have more Other than this ask can fulfill, so we take it all and move on.
// //                 fulfilled_amount += this_vol;
// //                 remaining_order_amount -= (this_vol as f64 * pv.price) as u64;
// //                 pv.volume = 0;
// //             } else {
// //                 // We have less Other than this ask can fulfill, so we take it and stop
// //                 pv.volume -= other_amount_requested;
// //                 remaining_order_amount = 0;
// //                 fulfilled_amount += other_amount_requested;
// //                 break
// //             }
// //         };
// //
// //         updated_curve.retain(|v| v.volume > 0);
// //
// //         if fulfilled_amount < DUST_LIMIT {
// //             None
// //         } else {
// //             Some(OrderFulfillment {
// //                 order_amount,
// //                 fulfilled_amount,
// //                 updated_curve,
// //                 is_ask_fulfillment_from_external_deposit: is_ask,
// //                 event_time,
// //                 tx_id_ref: tx_id.map(|id| ExternalTransactionId{ identifier: id, currency: SupportedCurrency::Bitcoin as i32 }),
// //                 destination: destination.clone(),
// //             })
// //         }
// //     }
// // }
