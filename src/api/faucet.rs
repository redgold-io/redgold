use std::collections::HashMap;
use std::time::Duration;
use itertools::Itertools;
use log::info;
use redgold_data::DataStoreContext;
use redgold_schema::{error_info, KeyPair, ProtoHashable, SafeOption, WithMetadataHashable};
use redgold_schema::structs::{Address, ErrorInfo, FaucetResponse, PublicResponse, Response, TransactionAmount};
use redgold_schema::transaction::{amount_data, amount_to_raw_amount};
use redgold_schema::transaction_builder::TransactionBuilder;
use crate::api::public_api::PublicClient;
use crate::e2e::tx_gen::{SpendableUTXO, TransactionGenerator};
use crate::core::internal_message::{RecvAsyncErrorInfo, SendErrorInfo, TransactionMessage};
use crate::core::relay::Relay;
use redgold_schema::EasyJson;

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
    info!("Incoming faucet request on {}", address_input);
    let node_config = relay.node_config.clone();

    let min_offset = 1;
    let max_offset = 5;

    let mut map: HashMap<Vec<u8>, KeyPair> = HashMap::new();
    for i in min_offset..max_offset {
        let key = node_config.internal_mnemonic().key_at(i);
        let address = key.address_typed();
        map.insert(address.address.unwrap().value, key);
    }

    let addresses = map.keys().map(|k| Address::from_bytes(k.clone()).unwrap()).collect_vec();

    let store = relay.clone().ds;
    let result = DataStoreContext::map_err_sqlx(store.query_utxo_address(addresses).await)?;
    // info!("Result here: {:?}", result);
    let utxos = result
        .iter()
        .map(|u| SpendableUTXO {
            utxo_entry: u.clone(),
            key_pair: map.get(&*u.address).expect("Map missing entry").clone(),
        })
        .collect_vec();
    //
    // let generator = TransactionGenerator::default_adv(
    //     utxos.clone(), min_offset, max_offset, node_config.internal_mnemonic()
    // );


    if utxos.is_empty() {
        Err(error_info("No UTXOs found for faucet"))
    } else {

        // TODO: We need to know this address is not currently in use -- i.e. local locker around
        // utxo in use.
        let utxo = utxos.get(0).safe_get()?.clone().clone();
        let addr = Address::parse(&address_input)?;
        let mut builder = TransactionBuilder::new();
        let transaction = builder
            .with_utxo(&utxo.utxo_entry)?
            .with_output(&addr, &TransactionAmount::from_fractional(5 as f64)?)
            .build()?
            .sign(&utxo.key_pair)?;

        info!("Faucet TX {}", transaction.json_or());

        let r_err = relay.submit_transaction_sync(&transaction).await?;

        let mut faucet_response = FaucetResponse::default();
        faucet_response.submit_transaction_response = Some(r_err);
        Ok(faucet_response)
    }
}
