use crate::e2e::tx_gen::SpendableUTXO;
use crate::util::{self};
use itertools::Itertools;
use log::{error, info};
use std::collections::HashMap;
use std::time::Duration;

use metrics::{counter, gauge};
use tokio_stream::wrappers::IntervalStream;
use redgold_schema::{RgResult, SafeOption};
use crate::core::internal_message::{RecvAsyncErrorInfo, SendErrorInfo};
use redgold_schema::structs::{Address, ErrorInfo, SubmitTransactionRequest, Transaction, CurrencyAmount};
use crate::core::relay::Relay;
use tokio_stream::StreamExt;
use redgold_keys::KeyPair;
use redgold_keys::transaction_support::{TransactionBuilderSupport, TransactionSupport};

pub mod tx_gen;
pub mod tx_submit;
pub mod alert;
use redgold_schema::EasyJson;
use redgold_schema::transaction::amount_to_raw_amount;
use redgold_schema::transaction_builder::TransactionBuilder;
use crate::util::logging::Loggable;
// i think this is the one currently in use?

// This one is NOT being used due to malfunctioning, thats why metrics weren't being picked up
// TODO: Debug why this isn't working as a local request?
// #[allow(dead_code)]
// pub fn run_remote(relay: Relay) {
//     let node_config = relay.node_config.clone();
//     info!("Starting e2e against local public API");
//     let runtime = util::runtimes::build_runtime(4, "e2e");
//
//     // runtime.spawn(auto_update::from_node_config(node_config.clone()));
//
//     let mut map: HashMap<Vec<u8>, KeyPair> = HashMap::new();
//     for i in 500..510 {
//         let key = node_config.internal_mnemonic().key_at(i);
//         let address = key.address();
//         map.insert(address, key);
//     }
//     let addresses = map.keys().map(|k| k.clone()).collect_vec();
//     let client: PublicClient = PublicClient {
//         url: "localhost".to_string(),
//         port: node_config.public_port(),
//         timeout: Duration::from_secs(90),
//     };
//
//     let result = runtime.block_on(client.query_addresses(addresses));
//     info!("Result here: {:?}", result);
//     let res = result.expect("Failed to query addresses on e2e startup");
//     let utxos = res
//         .query_addresses_response
//         .expect("No data on query address")
//         .utxo_entries
//         .iter()
//         .map(|u| SpendableUTXO {
//             utxo_entry: u.clone(),
//             key_pair: map.get(&*u.address).expect("Map missing entry").clone(),
//         })
//         .collect_vec();
//
//     let submit = TransactionSubmitter::default_adv(
//         client.clone(), runtime.clone(), utxos.clone(), 500, 510, node_config.internal_mnemonic()
//     );
//
//     let mut last_failure = util::current_time_millis();
//     let mut failed_once = false;
//     let mut num_success = 0 ;
//
//     if utxos.is_empty() {
//         // TODO: Faucet here from seed node.
//         info!("Unable to start e2e, no UTXOs")
//     } else {
//         loop {
//             info!("Canary submit");
//             sleep(Duration::from_secs(20));
//             let response = submit.submit().await;
//             if !response.is_ok() {
//                 counter!("redgold.e2e.failure").increment(1);
//                 let failure_msg = serde_json::to_string(&response).unwrap_or("ser failure".to_string());
//                 error!("Canary failure: {}", failure_msg.clone());
//                 let recovered = (num_success > 10 && util::current_time_millis() - last_failure > 1000 * 60 * 30);
//                 if !failed_once || recovered {
//                     alert::email(format!("{} e2e failure", relay.node_config.network.to_std_string()), &failure_msg);
//                 }
//                 let failure_time = util::current_time_millis();
//                 num_success = 0;
//                 last_failure = failure_time;
//                 failed_once = true;
//             } else {
//                 num_success += 1;
//                 info!("Canary success");
//                 counter!("redgold.e2e.success").increment(1);
//                 if let Some(s) = response.ok() {
//                     if let Some(q) = s.query_transaction_response {
//                         increment_gauge!("redgold.e2e.num_peers", q.observation_proofs.len() as f64);
//                     }
//                 }
//             }
//         }
//     }
// }
//

// async fn do_nothing() -> usize {
//     println!("yo");
//     return 1;
// }
//
// fn runtime_debug(j: usize) -> usize {
//     let rt2 = util::runtimes::build_runtime(1, "test23");
//     let result = rt2.block_on(do_nothing());
//     result
// }
//
// #[test]
// fn thread_test() {
//     // This doesn't work properly if we use spawn on the outer thread.
//     let rt = util::runtimes::build_runtime(1, "test");
//     let r = rt.spawn_blocking(|| runtime_debug(1));
//     let res = rt.block_on(r);
//     println!("res {:?}", res);
// }


struct LiveE2E {
    relay: Relay,
    last_failure: i64,
    failed_once: bool,
    num_success: u64,
}


impl LiveE2E {
    pub async fn build_tx(&self) -> RgResult<Transaction> {

        let mut map: HashMap<Address, KeyPair> = HashMap::new();
        if !self.relay.node_config.network.is_main() {
            let min_offset = 20;
            let max_offset = 30;
            for i in min_offset..max_offset {
                let key = self.relay.node_config.internal_mnemonic().key_at(i);
                let address = key.address_typed();
                map.insert(address, key);
            }
        } else {
            let key = self.relay.node_config.words().default_kp()?;
            let address = key.address_typed();
            map.insert(address, key);
        }
        let addresses = map.keys().map(|a| a.clone()).collect_vec();
        let mut utxo: Option<SpendableUTXO> = None;
        for (a, k) in map.iter() {
            let result = self.relay.ds.transaction_store.query_utxo_address(a).await?;
            let vec = result.iter().filter(|r| r.amount() > amount_to_raw_amount(1)).collect_vec();
            let utxo_m = vec.get(0);
            if let Some(u) = utxo_m {
                utxo = Some(SpendableUTXO {
                    utxo_entry: u.clone().clone(),
                    key_pair: k.clone(),
                });
                break;
            }
        }
        let u = utxo.safe_get_msg("No utxo in e2e")?;
        let mut tx_b = TransactionBuilder::new();
        let destination = addresses.iter()
            .find(|a| u.key_pair.address_typed() != a.clone().clone())
            .safe_get_msg("No destination address")?.clone().clone();
        let amount = CurrencyAmount::from_fractional(1f64).expect("");
        let tx = tx_b
            .with_output(&destination, &amount)
            .with_unsigned_input(u.utxo_entry.clone())?
            .with_is_test()
            .build()?
            .sign(&u.key_pair)?;
        return Ok(tx);
    }
}

pub async fn run(relay: Relay) -> Result<(), ErrorInfo> {
    run_wrapper(relay).await.log_error()
}

#[allow(dead_code)]
pub async fn run_wrapper(relay: Relay) -> Result<(), ErrorInfo> {
    let c = LiveE2E {
        relay: relay.clone(),
        last_failure: util::current_time_millis_i64(),
        failed_once: false,
        num_success: 0,
    };

    tokio::time::sleep(Duration::from_secs(100)).await;
    // See if we should start at all, but with a retry for genesis stuff
    c.build_tx().await?;

    let interval1 = tokio::time::interval(relay.node_config.clone().live_e2e_interval.clone());
    use futures::TryStreamExt;
    IntervalStream::new(interval1)
        .map(|x| Ok(x))
        .try_fold(c, |mut c, _| async {
            e2e_tick(&mut c).await.map(|_| c)
    }).await.map(|_x| ())
}

async fn e2e_tick(c: &mut LiveE2E) -> Result<(), ErrorInfo> {
    let result1 = c.build_tx().await;
    let result = result1.log_error().clone();
    match result {
        Err(_e) => {}
        Ok(transaction) => {
            let transaction = transaction.clone();
            let res = c.relay.submit_transaction(SubmitTransactionRequest {
                transaction: Some(transaction.clone()),
                sync_query_response: true
            }).await;

            match res {
                Ok(response) => {
                    c.num_success += 1;
                    info!("Live E2E request success");
                    counter!("redgold.e2e.success").increment(1);
                    if let Some(q) = response.query_transaction_response {
                        gauge!("redgold.e2e.num_peers").set(q.observation_proofs.len() as f64);
                    }
                }
                Err(e) => {
                    counter!("redgold.e2e.failure").increment(1);
                    let failure_msg = serde_json::to_string(&e).unwrap_or("ser failure".to_string());
                    error!("Canary failure: {}", failure_msg.clone());
                    let recovered = c.num_success > 10 && util::current_time_millis_i64() - c.last_failure > 1000 * 60 * 30;
                    if !c.failed_once || recovered {
                        alert::email(format!("{} e2e failure", c.relay.node_config.network.to_std_string()), &failure_msg).await?;
                    }
                    let failure_time = util::current_time_millis_i64();
                    c.num_success = 0;
                    c.last_failure = failure_time;
                    c.failed_once = true;
                }
            }
        }
    }
    Ok(())
}
