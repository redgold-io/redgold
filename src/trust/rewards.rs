// use std::sync::Arc;
// use std::time::Duration;
//
// use tokio::runtime::Runtime;
//
// use crate::core::relay::Relay;
// use crate::data::data_store::RewardQueryResult;
// use crate::schema::structs::{Input, Output, Transaction};
// use crate::schema::transaction::amount_data;
// use crate::util::current_time_unix;
// use redgold_schema::constants::{MIN_FEE_RAW, REWARD_AMOUNT_RAW};
// use redgold_schema::structs::Address;
//
// #[allow(dead_code)]
// #[derive(Clone)]
// pub struct Rewards {
//     relay: Relay,
//     moon_times: Vec<u64>,
//     next_reward: u64,
// }
//
// impl Rewards {
//     fn generate_reward_transaction(&self) {
//         let _latest = self.relay.ds.select_latest_reward_hash().unwrap();
//         let _input = Input {
//             transaction_hash: None,
//             output_index: 0,
//             proof: vec![],
//             product_id: None,
//             output: None,
//         };
//         let weights = self.relay.ds.select_reward_weights().unwrap();
//         let sum = weights
//             .clone()
//             .iter()
//             .map(|x| x.deterministic_trust)
//             .sum::<f64>();
//         let filtered = weights
//             .iter()
//             .filter(|x| {
//                 (x.deterministic_trust / sum) * (REWARD_AMOUNT_RAW as f64) > (MIN_FEE_RAW as f64)
//             })
//             .collect::<Vec<&RewardQueryResult>>();
//         let sum2 = filtered
//             .clone()
//             .iter()
//             .map(|x| x.deterministic_trust)
//             .sum::<f64>();
//         let rounded = filtered
//             .clone()
//             .iter()
//             .map(|x| ((x.deterministic_trust.clone() / sum2) * (REWARD_AMOUNT_RAW as f64)) as u64)
//             .sum::<u64>() as i64;
//         let delta = REWARD_AMOUNT_RAW - rounded;
//
//         let _outputs = filtered
//             .clone()
//             .iter()
//             .enumerate()
//             .map(|(idx, x)| {
//                 let mut _rounding = 0 as u64;
//                 if idx == 0 {
//                     _rounding = delta as u64;
//                 }
//                 let amount =
//                     ((x.deterministic_trust.clone() / sum2) * (REWARD_AMOUNT_RAW as f64)) as u64;
//                 Output {
//                     address: Address::address_data(x.reward_address.clone()),
//                     product_id: None,
//                     counter_party_proofs: vec![],
//                     data: amount_data(amount + (delta as u64)),
//                     contract: None,
//                 }
//             })
//             .collect::<Vec<Output>>();
//         // let _transaction = Transaction {
//         //     inputs: vec![input],
//         //     outputs,
//         //     options: None,
//         // };
//     }
//
//     async fn run(&mut self) {
//         let mut interval = tokio::time::interval(Duration::from_secs(
//             self.relay.node_config.reward_poll_interval_secs,
//         ));
//         loop {
//             interval.tick().await;
//             if current_time_unix() as u64 > self.next_reward {
//                 let _transaction = self.generate_reward_transaction();
//             }
//         }
//     }
//     // https://stackoverflow.com/questions/63347498/tokiospawn-borrowed-value-does-not-live-long-enough-argument-requires-tha
//     pub fn new(relay: Relay, arc: Arc<Runtime>) {
//         let vec = crate::trust::moon::load_moons();
//         let mut b = Self {
//             relay,
//             moon_times: vec.clone(),
//             next_reward: crate::trust::moon::next_reward_time(vec.clone()),
//         };
//         arc.spawn(async move { b.run().await });
//     }
// }
