use redgold_keys::KeyPair;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::util::btc_wallet::SingleKeyBitcoinWallet;
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_schema::{EasyJson, SafeOption};
use redgold_schema::structs::{CurrencyAmount, NetworkEnvironment, PublicKey};
use crate::core::transact::tx_builder_supports::{TransactionBuilder, TransactionBuilderSupport};
use crate::node_config::NodeConfig;

// Use this for testing AMM transactions.

pub fn amm_btc_address(network_environment: NetworkEnvironment) -> String {
    match network_environment {
        NetworkEnvironment::Dev => { "tb1qyxzxhpdkfdd9f2tpaxehq7hc4522f343tzgvt2" }
        NetworkEnvironment::Staging => { "tb1qftm7w70xmr7xplzp6w6ntxs4dygxtd2evc7x2e" }
        _ => { panic!("not implemented"); }
    }.to_string()
}
pub fn amm_public_key(network_environment: &NetworkEnvironment) -> PublicKey {
    let pk_hex = match network_environment {
        // NetworkEnvironment::Main => {}
        // NetworkEnvironment::Test => {}
        NetworkEnvironment::Dev => {"03879516077881c5be714024099c16974910d48b691c94c1824fad9635c17f3c37"}
        NetworkEnvironment::Staging => {"02efbe0b97d823da74ef2d88b2321d4e52ce2f62b137b0b5c5b415be9e40a9ca15"}
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
    let amm_addr = amm_public_key(&network).address().expect("address");
    let amount = 1.0;

    let addr = amm_addr.render_string().expect("");

    println!("sending to rdg amm address: {addr}");

    if let Some((_privk, keypair)) = dev_ci_kp() {
        let pk = keypair.public_key();
        let rdg_address = pk.address().expect("");
        println!("pk: {}", rdg_address.render_string().expect(""));

        let client = NodeConfig::default_env(network).await.api_client();
        client.faucet(&rdg_address).await.expect("faucet");
        let result = client.query_address(vec![rdg_address.clone()]).await.expect("").as_error().expect("");
        let utxos = result.query_addresses_response.safe_get_msg("missing query_addresses_response").expect("")
            .utxo_entries.clone();

        let amount = CurrencyAmount::from_fractional(amount).expect("");
        let tb = TransactionBuilder::new(&network)
            .with_network(&network)
            .with_utxos(&utxos).expect("utxos")
            .with_output(&amm_addr, &amount)
            .with_last_output_withdrawal_swap()
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
    // let a = w.address().expect("a");
    // println!("wallet address: {a}");
    // let b = w.get_wallet_balance().expect("balance");
    // println!("wallet balance: {b}");
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
