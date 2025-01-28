use crate::core::relay::Relay;
use crate::e2e::tx_gen::SpendableUTXO;
use itertools::Itertools;
use metrics::counter;
use redgold_common_no_wasm::tx_new::TransactionBuilderSupport;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::util::mnemonic_support::MnemonicSupport;
use redgold_keys::word_pass_support::WordsPassNodeConfig;
use redgold_keys::KeyPair;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::structs::{Address, CurrencyAmount, ErrorInfo, FaucetRequest, FaucetResponse};
use redgold_schema::tx::tx_builder::TransactionBuilder;
use redgold_schema::{error_info, ErrorInfoContext, SafeOption};
use reqwest::ClientBuilder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::info;
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

pub async fn faucet_request(faucet_request: &FaucetRequest, relay: &Relay, origin: Option<&String>) -> Result<FaucetResponse, ErrorInfo> {
    if relay.node_config.network.is_main() {
        return Err(error_info("Faucet not supported on mainnet"))
    }
    let option_token = faucet_request.token.clone();
    let faucet_addr = faucet_request.address.clone();
    let addr = faucet_addr.safe_get_msg("No address found")?;


    // info!("Incoming faucet request on {}", address_input);
    let node_config = relay.node_config.clone();

    let min_offset = 1;
    let max_offset = 5;

    let mut map: HashMap<Address, KeyPair> = HashMap::new();
    for i in min_offset..max_offset {
        let key = node_config.words().keypair_at_change(i).expect("works");
        let address = key.address_typed();
        // info!("Querying faucet address: {}", &address.json_or());
        map.insert(address, key);
    }

    let addresses = map.keys().map(|k| k.clone()).collect_vec();

    let store = relay.clone().ds;
    let result = store.transaction_store.utxo_for_addresses(&addresses.clone()).await?;
    // info!("Result here: {:?}", result);
    let utxos = result
        .iter()
        .map(|u| {
            let address = &u.address().expect("a");
            SpendableUTXO {
                utxo_entry: u.clone(),
                key_pair: map.get(address).expect("Map missing entry").clone(),
            }
        })
        .collect_vec();
    //
    // let generator = TransactionGenerator::default_adv(
    //     utxos.clone(), min_offset, max_offset, node_config.internal_mnemonic()
    // );


    if utxos.is_empty() {
        Err(error_info("No UTXOs found for faucet"))
    } else {

        let mut amount = 5.0f64;
        if relay.node_config.network.is_main_stage_network() {
            let origin = *origin.safe_get_msg("No origin found")?;
            if !relay.check_rate_limit(origin)? {
                return Err(error_info("Rate limit exceeded"));
            }
            let token = option_token.safe_get_msg("No recaptcha token found")?;
            let captcha = recaptcha_verify(token.clone(), None, Some(origin.clone())).await?;
            if !captcha {
                return Err(error_info("Recaptcha verification failed"));
            }
            amount = 0.05f64;
        }

        // TODO: We need to know this address is not currently in use -- i.e. local locker around
        // utxo in use.
        let utxo = utxos.get(0).safe_get()?.clone().clone();
        let mut builder = TransactionBuilder::new(&relay.node_config);
        let transaction = builder
            .with_utxo(&utxo.utxo_entry)?
            .with_output(&addr, &CurrencyAmount::from_fractional(amount)?)
            .with_message("faucet")?
            .build()?
            .sign(&utxo.key_pair)?;

        info!("Faucet TX {}", transaction.json_or());
        counter!("redgold.faucet").increment(1);

        let r_err = relay.submit_transaction_sync(&transaction).await?;

        let mut faucet_response = FaucetResponse::default();
        faucet_response.submit_transaction_response = Some(r_err);
        Ok(faucet_response)
    }
}


#[derive(Debug, Serialize)]
struct RecaptchaRequestBody {
    secret: String,
    response: String,
    remoteip: Option<String>, // Optional, include this only if you want to verify the user's IP
}

#[derive(Debug, Deserialize)]
struct RecaptchaResponse {
    success: bool,
    // You can add more fields based on what you need from the response
}

pub async fn recaptcha_verify(token: String, secret: Option<String>, remoteip: Option<String>) -> Result<bool, ErrorInfo> {
    let result = std::env::var("RECAPTCHA_SECRET").ok();
    let secret = secret.or(result).ok_msg("No recaptcha secret found")?;
    let client = ClientBuilder::new().timeout(Duration::from_secs(60)).build()
        .map_err(|e| ErrorInfo::new(format!("Failed to build client for recaptcha verify: {}", e)))?;

    let body = RecaptchaRequestBody {
        secret,
        response: token,
        remoteip,
    };

    let response = client
        .post("https://www.google.com/recaptcha/api/siteverify")
        .form(&body)
        .send()
        .await
        .map_err(|e| ErrorInfo::new(format!("Query request failed: {}", e)))?;

    if response.status().is_success() {
        let recaptcha_response = response.json::<RecaptchaResponse>().await
            .map_err(|e| ErrorInfo::new(format!("Failed to parse response: {}", e)))?;
        Ok(recaptcha_response.success)
    } else {
        Err(ErrorInfo::new(format!("Recaptcha verification failed with status: {}", response.status())))
    }
}