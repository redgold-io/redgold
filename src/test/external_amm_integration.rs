use redgold_common_no_wasm::tx_new::TransactionBuilderSupport;
use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};

use redgold_keys::KeyPair;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::util::btc_wallet::SingleKeyBitcoinWallet;
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_rpc_integ::eth::eth_wallet::EthWalletWrapper;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{Address, CurrencyAmount, NetworkEnvironment, PublicKey};
use redgold_schema::util::lang_util::AnyPrinter;
use redgold_schema::tx::tx_builder::TransactionBuilder;
use crate::node_config::{EnvDefaultNodeConfig, ToTransactionBuilder};
use crate::core::transact::tx_broadcast_support::TxBroadcastSupport;
use crate::core::transact::tx_builder_supports::{TxBuilderApiConvert, TxBuilderApiSupport};
// Use this for testing AMM transactions.

pub fn amm_btc_address(network_environment: NetworkEnvironment) -> String {
    amm_public_key(&network_environment)
        .to_bitcoin_address_typed(&network_environment).expect("address").render_string().expect("address")
}
pub fn amm_public_key(network_environment: &NetworkEnvironment) -> PublicKey {
    let pk_hex = match network_environment {
        NetworkEnvironment::Main => {"0a230a210220f12e974037da99be8152333d4b72fc06c9041fbd39ac6b37fb6f65e3057c39"}
        NetworkEnvironment::Test => {"0a230a21034c16cf716ba671c85ccb68d597104ba608fe798e4c4e50eaa36ab68457c25ed8"}
        NetworkEnvironment::Dev => {"0a230a2103f952d12024fd41e9817470b6910d545799a0fabf6a1e40228ea91d9a330c051b"}
        NetworkEnvironment::Staging => {"0a230a210214e61824f16e43e769df927ec13148b9f8e9596800878fa76cef3edbc1eb5373"}
        _ => { panic!("not implemented"); }
    };
    PublicKey::from_hex(pk_hex.to_string()).expect("pk")
}

// Has faucet bitcoin test funds
pub fn dev_ci_kp() -> Option<(String, KeyPair)> {
    if let Some(w) = std::env::var("REDGOLD_TEST_WORDS").ok() {
        let (privk, keypair) = words_to_ci_keypair(w);
        Some((privk, keypair))
    } else {
        None
    }
}

pub fn words_to_ci_keypair(w: String) -> (String, KeyPair) {
    let w = WordsPass::new(w, None);
    let path = "m/84'/0'/0'/0/0";
    let privk = w.private_at(path.to_string()).expect("private key");
    let keypair = w.keypair_at(path.to_string()).expect("private key");
    (privk, keypair)
}

pub async fn send_btc(amount: i64, net: &NetworkEnvironment) {
    let (privk, kp) = dev_ci_kp().expect("keys");
    let pk = kp.public_key();
    let mut w =
        SingleKeyBitcoinWallet::new_wallet(pk, net.clone(), true)
            .expect("w");
    let a = w.address().expect("works");
    println!("wallet address: {a}");
    let b = w.get_wallet_balance().expect("balance");
    println!("wallet balance: {b}");
    let res = w.send_local(amm_btc_address(net.clone()), amount as u64, privk).expect("send");
    println!("txid: {res}");
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

//
// // Use this for testing AMM transactions.
// #[ignore]
// #[tokio::test]
// pub async fn send_test_rdg_btc_tx_withdrawal() {
//
//     let network = NetworkEnvironment::Staging;
//     let nc = NodeConfig::default_env(network).await;
//     let amm_addr = amm_public_key(&network).address().expect("address");
//     let amount = 1.0;
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
//
//     }
// }

#[ignore]
#[test]
pub fn dev_balance_check() {

    let network = NetworkEnvironment::Main;
    let pk = amm_public_key(&network);

    let addr = pk.address().expect("address").render_string().expect("");

    // let xpub = "testmehere";
    // let pk2 = XpubWrapper::new(xpub.to_string()).public_at(0, 0).expect("works");

    // println!("address: {addr}");

    let w =
        SingleKeyBitcoinWallet::new_wallet(pk, network, true).expect("w");

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








pub async fn send_internal_stake(amt: f64, network: &NetworkEnvironment) {
    let nc = NodeConfig::default_env(network.clone()).await;
    let internal_stake_amount = CurrencyAmount::from_fractional(amt).expect("works");
    let (privk, kp) = dev_ci_kp().expect("keys");
    let pk = kp.public_key();
    let rdg_address = pk.address().expect("address");
    let amm_rdg_address = amm_public_key(&network).address().expect("address");
    let internal_stake_tx = TransactionBuilder::new(&nc)
        .with_input_address(&rdg_address).clone()
        .into_api_wrapper()
        .with_auto_utxos().await.expect("utxos")
        .with_internal_stake_usd_bounds(
            None, None, &rdg_address, &amm_rdg_address, &internal_stake_amount,
        )
        .build()
        .expect("build")
        .sign(&kp)
        .expect("sign");
    let response = internal_stake_tx.broadcast().await.expect("broadcast").json_or();
    println!("response: {response}");

}


#[ignore]
#[tokio::test]
pub async fn amm_flow() {
    // let network = NetworkEnvironment::Dev;
    let network = NetworkEnvironment::Main;
    // let amount_sats = 40000;

    let nc = NodeConfig::default_env(network).await;
    let amm_rdg_address = amm_public_key(&network).address().expect("address");
    let amm_eth_address = amm_public_key(&network).to_ethereum_address_typed().expect("address");
    let amm_btc_pk_address = amm_public_key(&network).to_bitcoin_address_typed(&network).expect("address");
    // let amount = 1.0;

    println!("amm rdg address: {}", amm_rdg_address.render_string().expect(""));
    println!("amm eth address: {}", amm_eth_address.render_string().expect(""));
    println!("amm btc address: {}", amm_btc_pk_address.render_string().expect(""));

    if let Some((privk, kp)) = dev_ci_kp() {

        // send_internal_stake(5500.0, &network).await;

        let pk = kp.public_key();
        let rdg_address = pk.address().expect("address");
        println!("pk rdg address: {}", rdg_address.render_string().expect(""));
        let btc_address = pk.to_bitcoin_address_typed(&network).expect("btc address");
        println!("pk btc address: {}", btc_address.render_string().expect(""));
        let eth_address = pk.to_ethereum_address_typed().expect("eth address");
        println!("pk eth address: {}", eth_address.render_string().expect(""));
        //
        let btc_stake_amt = 20_000;
        let btc_amt = CurrencyAmount::from_btc(btc_stake_amt);
        let btc_address = pk.to_bitcoin_address_typed(&network).expect("btc address");
        let party_fee_amount = CurrencyAmount::from_rdg(100000);
        // let stake_tx = TransactionBuilder::new(&nc)
        //     .with_input_address(&rdg_address)
        //     .with_auto_utxos().await.expect("utxos")
        //     .with_external_stake_usd_bounds(None, None, &rdg_address, &btc_address, &btc_amt, &amm_rdg_address, &party_fee_amount)
        //     .build()
        //     .expect("build")
        //     .sign(&kp)
        //     .expect("sign");
        // stake_tx.broadcast().await.expect("broadcast").json_or().print();

        // tokio::time::sleep(Duration::from_secs(5)).await;
        // send_btc(btc_stake_amt, &network).await;
        //
        let dev_ci_eth_addr = kp.public_key().to_ethereum_address_typed().expect("works");
        let exact_eth_stake_amount = CurrencyAmount::stake_test_amount_typed();
        let party_fee_amount = CurrencyAmount::from_rdg(100000);

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
        // stake_tx.broadcast().await.expect("broadcast").json_or().print();
        // tokio::time::sleep(Duration::from_secs(5)).await;

        let eth_submit = EthWalletWrapper::new(&privk, &network).expect("works");
        // eth_submit.send(&amm_eth_address, &exact_eth_stake_amount).await.expect("works").print();



        // External to internal swaps
        // eth_submit.send(&amm_eth_address, &CurrencyAmount::from_eth_fractional(0.0111)).await.expect("works").print();
        // eth_submit.send(&amm_eth_address, &CurrencyAmount::from_eth_fractional(0.002)).await.expect("works").print();
        // test btc swap
        // send_btc(20_004, &network).await;
        internal_to_external_swap(nc, &amm_rdg_address, &kp, &rdg_address, &btc_address, &dev_ci_eth_addr).await;

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

async fn internal_to_external_swap(nc: NodeConfig, amm_rdg_address: &Address, kp: &KeyPair,
                                   rdg_address: &Address, btc_address: &Address, dev_ci_eth_addr: &Address) {
    // test rdg->btc swap
    nc.tx_builder().with_input_address(&rdg_address)
        .clone()
        .into_api_wrapper()
        .with_auto_utxos().await.expect("utxos")
        .with_swap(&btc_address, &CurrencyAmount::from_fractional(0.05551).unwrap(), &amm_rdg_address)
        .unwrap()
        .build()
        .unwrap()
        .sign(&kp)
        .unwrap()
        .broadcast().await.expect("broadcast").json_pretty_or().print();

    // test rdg->eth swap
    nc.tx_builder().with_input_address(&rdg_address)
        .clone()
        .into_api_wrapper()
        .with_auto_utxos().await.expect("utxos")
        .with_swap(&dev_ci_eth_addr, &CurrencyAmount::from_fractional(0.05552).unwrap(), &amm_rdg_address)
        .unwrap()
        .build()
        .unwrap()
        .sign(&kp)
        .unwrap()
        .broadcast().await.expect("broadcast").json_pretty_or().print();
}
