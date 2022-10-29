use std::collections::HashMap;
use std::time::Duration;
use itertools::Itertools;
use log::info;
use redgold_data::DataStoreContext;
use redgold_schema::{error_info, KeyPair, WithMetadataHashable};
use redgold_schema::structs::{Address, ErrorInfo, FaucetResponse, PublicResponse};
use crate::api::public_api::PublicClient;
use crate::canary::tx_gen::{SpendableUTXO, TransactionGenerator};
use crate::core::internal_message::{RecvAsyncErrorInfo, TransactionMessage};
use crate::core::relay::Relay;
//
// async fn faucet_request_old(address_input: String, relay: Relay) -> Result<FaucetResponse, ErrorInfo> {
//     info!("Faucet request {}", address_input);
//     let node_config = relay.node_config;
//
//     // runtime.spawn(auto_update::from_node_config(node_config.clone()));
//
//     let mut map: HashMap<Vec<u8>, KeyPair> = HashMap::new();
//     for i in 0..100 {
//         let key = node_config.wallet().key_at(i);
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
//     let result = client.query_addresses(addresses).await?;
//     info!("Result here: {:?}", result);
//     let utxos = result
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
//     let submit = TransactionGenerator::default_adv(
//         utxos.clone(), 0, 100, node_config.wallet()
//     );
//
//     if utxos.is_empty() {
//         return Err(error_info("No UTXOs found for faucet"));
//     } else {
//
//     }
//     Ok(FaucetResponse{ transaction_hash: None })
// }

pub async fn faucet_request(address_input: String, relay: Relay) -> Result<FaucetResponse, ErrorInfo> {
    info!("Faucet request {}", address_input);
    let node_config = relay.node_config.clone();

    let mut map: HashMap<Vec<u8>, KeyPair> = HashMap::new();
    for i in 0..100 {
        let key = node_config.wallet().key_at(i);
        let address = key.address_typed();
        map.insert(address.address.unwrap().bytes_value, key);
    }

    let addresses = map.keys().map(|k| Address::from_bytes(k.clone()).unwrap()).collect_vec();

    let store = relay.clone().ds;
    let result = DataStoreContext::map_err_sqlx(store.query_utxo_address(addresses).await)?;
    info!("Result here: {:?}", result);
    let utxos = result
        .iter()
        .map(|u| SpendableUTXO {
            utxo_entry: u.clone(),
            key_pair: map.get(&*u.address).expect("Map missing entry").clone(),
        })
        .collect_vec();

    let mut generator = TransactionGenerator::default_adv(
        utxos.clone(), 0, 100, node_config.wallet()
    );


    if utxos.is_empty() {
        Err(error_info("No UTXOs found for faucet"))
    } else {

        let transaction = generator.generate_simple_tx().clone();
        let (sender, receiver) = flume::unbounded::<PublicResponse>();
        let message = TransactionMessage {
            transaction: transaction.transaction.clone(),
            response_channel: Some(sender)
        };
        relay
            .clone()
            .transaction
            .sender
            .send(message)
            .expect("send");

        let r_err = receiver.recv_async_err().await;

        match r_err {
            Ok(response) => {
                if response.clone().accepted() {
                    // generator.completed(transaction.clone());
                    Ok(FaucetResponse{ transaction_hash: Some(transaction.transaction.hash()) })
                } else {
                    let e = format!("Faucet failure: {:?}", serde_json::to_string(&response));
                    info!("Faucet failure: {}", e);
                    Err(error_info(e))
                }
            }
            Err(e) => Err(e)
        }
    }
}
