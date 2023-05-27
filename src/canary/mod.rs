use crate::api::public_api::PublicClient;
use crate::canary::tx_gen::{SpendableUTXO, TransactionGenerator};
use crate::canary::tx_submit::TransactionSubmitter;
use crate::node_config::NodeConfig;
use crate::util::{cli, init_logger};
use crate::util::{self, auto_update};
use itertools::Itertools;
use log::{error, info};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use metrics::{increment_counter, increment_gauge};
use tokio::runtime::Runtime;
use tokio_stream::wrappers::IntervalStream;
use redgold_schema::KeyPair;
use crate::core::internal_message::{RecvAsyncErrorInfo, SendErrorInfo, TransactionMessage};
use redgold_schema::structs::{Address, ErrorInfo, PublicResponse, SubmitTransactionRequest};
use crate::core::relay::Relay;
use crate::util::cli::args::{DebugCanary, RgTopLevelSubcommand};
use tokio_stream::{StreamExt};
pub mod tx_gen;
pub mod tx_submit;
pub mod alert;
use redgold_schema::EasyJson;
// i think this is the one currently in use?

// This one is NOT being used due to malfunctioning, thats why metrics weren't being picked up
// TODO: Debug why this isn't working as a local request?
// #[allow(dead_code)]
// pub fn run_remote(relay: Relay) {
//     let node_config = relay.node_config.clone();
//     info!("Starting canary against local public API");
//     let runtime = util::runtimes::build_runtime(4, "canary");
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
//     let res = result.expect("Failed to query addresses on canary startup");
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
//         info!("Unable to start canary, no UTXOs")
//     } else {
//         loop {
//             info!("Canary submit");
//             sleep(Duration::from_secs(20));
//             let response = submit.submit().await;
//             if !response.is_ok() {
//                 increment_counter!("redgold.canary.failure");
//                 let failure_msg = serde_json::to_string(&response).unwrap_or("ser failure".to_string());
//                 error!("Canary failure: {}", failure_msg.clone());
//                 let recovered = (num_success > 10 && util::current_time_millis() - last_failure > 1000 * 60 * 30);
//                 if !failed_once || recovered {
//                     alert::email(format!("{} canary failure", relay.node_config.network.to_std_string()), &failure_msg);
//                 }
//                 let failure_time = util::current_time_millis();
//                 num_success = 0;
//                 last_failure = failure_time;
//                 failed_once = true;
//             } else {
//                 num_success += 1;
//                 info!("Canary success");
//                 increment_counter!("redgold.canary.success");
//                 if let Some(s) = response.ok() {
//                     if let Some(q) = s.query_transaction_response {
//                         increment_gauge!("redgold.canary.num_peers", q.observation_proofs.len() as f64);
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

#[tokio::test]
async fn debug_local_test() {
    if !util::local_debug_mode() {
        return;
    }
    init_logger().expect("log");

    // let rt = util::runtimes::build_runtime(1, "test");
    let mut args = cli::args::empty_args();
    args.subcmd = Some(RgTopLevelSubcommand::DebugCanary(DebugCanary { host: None }));
    cli::arg_parse_config::load_node_config(args, NodeConfig::default()).await.expect("works");
    // let mut config = NodeConfig::default();
    // info!("Node config: {:?}", config.clone());
    // config.network_type = NetworkEnvironment::Local;
    // run(config)
}


struct Canary {
    relay: Relay,
    utxos: Vec<SpendableUTXO>,
    last_failure: i64,
    failed_once: bool,
    num_success: u64,
    generator: TransactionGenerator,
}

#[allow(dead_code)]
pub async fn run(relay: Relay) -> Result<(), ErrorInfo> {
    let node_config = relay.node_config.clone();
    sleep(Duration::from_secs(60));
    info!("Starting canary");
    let runtime = util::runtimes::build_runtime(4, "canary");

    // runtime.spawn(auto_update::from_node_config(node_config.clone()));

    let mut map: HashMap<Vec<u8>, KeyPair> = HashMap::new();
    let min_offset = 20;
    let max_offset = 30;
    for i in min_offset..max_offset {
        let key = node_config.internal_mnemonic().key_at(i);
        let address = key.address_typed();
        map.insert(address.address.unwrap().value, key);
    }
    let addresses = map.keys().map(|k| Address::from_bytes(k.clone()).unwrap()).collect_vec();
    // let client: PublicClient = PublicClient {
    //     url: "localhost".to_string(),
    //     port: node_config.public_port(),
    //     timeout: Duration::from_secs(90),
    // };
    //
    //

    // let result = runtime.block_on(client.query_addresses(addresses));
    let result = runtime.block_on(relay.clone().ds.query_utxo_address(addresses));
    if let Err(e) = &result {
        error!("Canary query utxo failure {}", e.to_string());
    }

    let res = result.unwrap_or(vec![]);
    let utxos = res
        .iter()
        .map(|u| SpendableUTXO {
            utxo_entry: u.clone(),
            key_pair: map.get(&*u.address).expect("Map missing entry").clone(),
        })
        .collect_vec();

    let generator = TransactionGenerator::default_adv(
        utxos.clone(), min_offset, max_offset, node_config.internal_mnemonic()
    );


    if utxos.is_empty() {
        // TODO: Faucet here from seed node.
        info!("Unable to start canary, no UTXOs");
        Ok(())
    } else {
        // TODO Change to tokio
        let c = Canary {
            relay,
            utxos,
            generator,
            last_failure: util::current_time_millis_i64(),
            failed_once: false,
            num_success: 0
        };
        let interval1 = tokio::time::interval(Duration::from_secs(60));
        use futures::TryStreamExt;
        IntervalStream::new(interval1)
            .map(|x| Ok(x))
            .try_fold(c, |mut c, _| async {
                canary_tick(&mut c).await?;
                Ok(c)

        }).await?;
        Ok(())
    }
}

async fn canary_tick(c: &mut Canary) -> Result<(), ErrorInfo> {
    let transaction = c.generator.generate_simple_tx().clone();
    let res = c.relay.submit_transaction(SubmitTransactionRequest {
        transaction:
        Some(transaction.transaction.clone()),
        sync_query_response: true
    }).await;

    match res {
        Ok(response) => {
            c.num_success += 1;
            info!("Canary success");
            increment_counter!("redgold.canary.success");
            if let Some(q) = response.query_transaction_response {
                increment_gauge!("redgold.canary.num_peers", q.observation_proofs.len() as f64);
            }
        }
        Err(e) => {
            increment_counter!("redgold.canary.failure");
            let failure_msg = serde_json::to_string(&e).unwrap_or("ser failure".to_string());
            error!("Canary failure: {}", failure_msg.clone());
            let recovered = c.num_success > 10 && util::current_time_millis_i64() - c.last_failure > 1000 * 60 * 30;
            if !c.failed_once || recovered {
                alert::email(format!("{} canary failure", c.relay.node_config.network.to_std_string()), &failure_msg).await?;
            }
            let failure_time = util::current_time_millis_i64();
            c.num_success = 0;
            c.last_failure = failure_time;
            c.failed_once = true;
        }
    }
    Ok(())
}
