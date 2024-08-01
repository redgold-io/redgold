use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};
use redgold_keys::eth::eth_wallet::EthWalletWrapper;
use redgold_keys::KeyPair;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::util::btc_wallet::SingleKeyBitcoinWallet;
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_schema::SafeOption;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{CurrencyAmount, NetworkEnvironment, PublicKey};
use crate::core::transact::tx_builder_supports::{TransactionBuilder, TransactionBuilderSupport};
use crate::node_config::NodeConfig;
use crate::core::transact::tx_broadcast_support::TxBroadcastSupport;
// Use this for testing AMM transactions.

pub fn amm_btc_address(network_environment: NetworkEnvironment) -> String {
    match network_environment {
        NetworkEnvironment::Dev => { "tb1q4zkz4qlkdkwhwry88a4pk82plcp5fnzmh2y53w" }
        NetworkEnvironment::Staging => { "tb1qftm7w70xmr7xplzp6w6ntxs4dygxtd2evc7x2e" }
        _ => { panic!("not implemented"); }
    }.to_string()
}
pub fn amm_public_key(network_environment: &NetworkEnvironment) -> PublicKey {
    let pk_hex = match network_environment {
        // NetworkEnvironment::Main => {}
        // NetworkEnvironment::Test => {}
        NetworkEnvironment::Dev => {"0a230a210266f48bc55acec1647168d40fe827359f9b1f8ca457a0c6b111a1881f84aaea46"}
        // NetworkEnvironment::Staging => {"02efbe0b97d823da74ef2d88b2321d4e52ce2f62b137b0b5c5b415be9e40a9ca15"}
        _ => { panic!("not implemented"); }
    };
    PublicKey::from_hex(pk_hex.to_string()).expect("pk")
}

// Has faucet bitcoin test funds
pub fn dev_ci_kp() -> Option<(String, KeyPair)> {
    if let Some(w) = std::env::var("REDGOLD_TEST_WORDS").ok() {
        let w = WordsPass::new(w, None);
        let path = "m/84'/0'/0'/0/0";
        let privk = w.private_at(path.to_string()).expect("private key");
        let keypair = w.keypair_at(path.to_string()).expect("private key");
        Some((privk, keypair))
    } else {
        None
    }
}

#[ignore]
#[tokio::test]
pub async fn debug_kp() {
    if let Some(w) = std::env::var("REDGOLD_TEST_WORDS").ok() {
        let path = "m/84'/0'/0'/0/0".to_string();
        let w = WordsPass::words(w);
        // w.xpub()
    }
}

#[ignore]
#[tokio::test]
pub async fn send_test_btc_transaction_deposit() {
    let network = NetworkEnvironment::Staging;
    let amount_sats = 40000;

    if let Some((privk, kp)) = dev_ci_kp() {
        let pk = kp.public_key();
        let mut w =
            SingleKeyBitcoinWallet::new_wallet(pk, NetworkEnvironment::Dev, true)
                .expect("w");
        let a = w.address().expect("a");
        println!("wallet address: {a}");
        let b = w.get_wallet_balance().expect("balance");
        println!("wallet balance: {b}");
        let res = w.send_local(amm_btc_address(network), amount_sats, privk).expect("send");
        println!("txid: {res}");
    }
}


// Use this for testing AMM transactions.
#[ignore]
#[tokio::test]
pub async fn send_test_rdg_btc_tx_withdrawal() {

    let network = NetworkEnvironment::Staging;
    let nc = NodeConfig::default_env(network).await;
    let amm_addr = amm_public_key(&network).address().expect("address");
    let amount = 1.0;

    let addr = amm_addr.render_string().expect("");

    println!("sending to rdg amm address: {addr}");

    if let Some((_privk, keypair)) = dev_ci_kp() {
        let pk = keypair.public_key();
        let rdg_address = pk.address().expect("");
        println!("pk: {}", rdg_address.render_string().expect(""));

        let client = nc.api_client();
        client.faucet(&rdg_address).await.expect("faucet");
        let result = client.query_address(vec![rdg_address.clone()]).await.expect("").as_error().expect("");
        let utxos = result.query_addresses_response.safe_get_msg("missing query_addresses_response").expect("")
            .utxo_entries.clone();

        let btc_addr = pk.to_bitcoin_address_typed(&network).expect("btc address");

        let amount = CurrencyAmount::from_fractional(amount).expect("");
        let tb = TransactionBuilder::new(&nc)
            .with_network(&network)
            .with_utxos(&utxos).expect("utxos")
            .with_output(&amm_addr, &amount)
            .with_last_output_swap_type()
            .with_last_output_swap_destination(&btc_addr).expect("swap")
            .build().expect("build")
            .sign(&keypair).expect("sign");

        let res = client.send_transaction(&tb, true).await.expect("send").json_or();

        println!("send: {}", res);

    }
}

#[ignore]
#[test]
pub fn dev_balance_check() {

    let network = NetworkEnvironment::Dev;
    let pk = amm_public_key(&network);

    let addr = pk.address().expect("address").render_string().expect("");

    println!("address: {addr}");

    let w =
        SingleKeyBitcoinWallet::new_wallet(pk, network, true).expect("w");

    //
    let a = w.address().expect("a");
    println!("wallet address: {a}");
    let b = w.get_wallet_balance().expect("balance");
    println!("wallet balance: {b}");
    //
    // let mut tx = w.get_sourced_tx().expect("sourced tx");
    // tx.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    // for t in tx {
    //     let tj = t.json_or();
    //     println!("tx: {tj}");
    // }

    println!("now the new function showing both");

    let mut tx2 = w.get_all_tx().expect("sourced tx");
    tx2.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    for t in tx2 {
        let tj = t.json_or();
        println!("tx: {tj}");
    }


}










#[ignore]
#[tokio::test]
pub async fn send_test_btc_staking_tx() {
    let network = NetworkEnvironment::Dev;
    // let amount_sats = 40000;

    let nc = NodeConfig::default_env(network).await;
    let amm_rdg_address = amm_public_key(&network).address().expect("address");
    let amm_eth_address = amm_public_key(&network).to_ethereum_address_typed().expect("address");
    let amm_btc_pk_address = amm_public_key(&network).to_bitcoin_address_typed(&network).expect("address");
    // let amount = 1.0;

    if let Some((privk, kp)) = dev_ci_kp() {
        let pk = kp.public_key();
        let rdg_address = pk.address().expect("address");
        println!("pk rdg address: {}", rdg_address.render_string().expect(""));

        let mut w =
            SingleKeyBitcoinWallet::new_wallet(pk.clone(), NetworkEnvironment::Dev, true)
                .expect("w");
        let a = w.address().expect("a");
        println!("wallet address: {a}");
        let b = w.get_wallet_balance().expect("balance");
        println!("wallet balance: {b}");
        // wallet balance: { immature: 0, trusted_pending: 0, untrusted_pending: 0, confirmed: 3818590 }
        //
        let btc_amt = CurrencyAmount::from_btc(50_000);
        let btc_address = pk.to_bitcoin_address_typed(&network).expect("btc address");
        let party_fee_amount = CurrencyAmount::from_rdg(100000);
        // let stake_tx = TransactionBuilder::new(&nc)
        //     .with_input_address(&rdg_address)
        //     .with_auto_utxos().await.expect("utxos")
        //     .with_external_stake_usd_bounds(None, None, &rdg_address, &btc_address, &btc_amt, &amm_addr, &party_fee_amount)
        //     .build()
        //     .expect("build")
        //     .sign(&kp)
        //     .expect("sign");
        // let response = stake_tx.broadcast().await.expect("broadcast").json_or();
        // println!("response: {response}");
        //
        //
        // let res = w.send_local(amm_btc_address(network), 50_000, privk).expect("send");
        // println!("txid: {res}");
        // let internal_stake_amount = CurrencyAmount::from_fractional(100).expect("works");
        //
        // let internal_stake_tx = TransactionBuilder::new(&nc)
        //     .with_input_address(&rdg_address)
        //     .with_auto_utxos().await.expect("utxos")
        //     .with_internal_stake_usd_bounds(
        //         None, None, &rdg_address, &amm_addr, &internal_stake_amount,
        //     )
        //     .build()
        //     .expect("build")
        //     .sign(&kp)
        //     .expect("sign");
        // let response = internal_stake_tx.broadcast().await.expect("broadcast").json_or();
        // println!("response: {response}");
        //


        let dev_ci_eth_addr = kp.public_key().to_ethereum_address_typed().expect("works");
        let exact_eth_stake_amount = EthWalletWrapper::stake_test_amount_typed();
        let party_fee_amount = CurrencyAmount::from_rdg(100000);
        //
        // let stake_tx = TransactionBuilder::new(&nc)
        //     .with_input_address(&rdg_address)
        //     .with_auto_utxos().await.expect("utxos")
        //     .with_external_stake_usd_bounds(
        //         None, None, &rdg_address, &dev_ci_eth_addr, &exact_eth_stake_amount, &amm_rdg_address, &party_fee_amount
        //     )
        //     .build()
        //     .expect("build")
        //     .sign(&kp)
        //     .expect("sign");
        // let response = stake_tx.broadcast().await.expect("broadcast").json_or();
        // println!("response: {response}");
        // info!("tx_stake tx time: {}", tx_stake.time().expect("time").to_string());
        // info!("tx_stake tx: {}", tx_stake.json_or());
        //
        // let eth_submit = EthWalletWrapper::new(&privk, &network).expect("works");
        // let res = eth_submit.send(&amm_eth_address, &exact_eth_stake_amount).await.expect("works");
        // println!("eth tx: {res}");

        // test btc swap

        let btc_swap_amt = CurrencyAmount::from_btc(10_000);
        let res = w.send_local(amm_btc_pk_address.render_string().unwrap(), 10_000, privk).expect("send");
        println!("txid: {res}");


    }
//     }
// }
//
//
//     let addr = amm_addr.render_string().expect("");
//
//     println!("sending to rdg amm address: {addr}");
//
//     if let Some((_privk, keypair)) = dev_ci_kp() {
//         let pk = keypair.public_key();
//         let rdg_address = pk.address().expect("");
//         println!("pk: {}", rdg_address.render_string().expect(""));
//
//         let client = nc.api_client();
//         client.faucet(&rdg_address).await.expect("faucet");
//         let result = client.query_address(vec![rdg_address.clone()]).await.expect("").as_error().expect("");
//         let utxos = result.query_addresses_response.safe_get_msg("missing query_addresses_response").expect("")
//             .utxo_entries.clone();
//
//         let btc_addr = pk.to_bitcoin_address_typed(&network).expect("btc address");
//
//         let amount = CurrencyAmount::from_fractional(amount).expect("");
//         let tb = TransactionBuilder::new(&nc)
//             .with_network(&network)
//             .with_utxos(&utxos).expect("utxos")
//             .with_output(&amm_addr, &amount)
//             .with_last_output_swap_type()
//             .with_last_output_swap_destination(&btc_addr).expect("swap")
//             .build().expect("build")
//             .sign(&keypair).expect("sign");
//
//         let res = client.send_transaction(&tb, true).await.expect("send").json_or();
//
//         println!("send: {}", res);
}
