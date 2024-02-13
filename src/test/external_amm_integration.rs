use redgold_keys::transaction_support::{TransactionBuilderSupport, TransactionSupport};
use redgold_keys::util::btc_wallet::SingleKeyBitcoinWallet;
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_schema::{EasyJson, SafeOption};
use redgold_schema::structs::{CurrencyAmount, NetworkEnvironment, PublicKey};
use redgold_schema::transaction_builder::TransactionBuilder;
use crate::node_config::NodeConfig;


// Use this for testing AMM transactions.
#[ignore]
#[tokio::test]
pub async fn load_ci_kp() {

    let dev_amm_address = "tb1qyxzxhpdkfdd9f2tpaxehq7hc4522f343tzgvt2".to_string();

    let pk_hex = "03879516077881c5be714024099c16974910d48b691c94c1824fad9635c17f3c37";
    let dev_amm_pk = PublicKey::from_hex(pk_hex).expect("pk");

    let dev_amm_rdg_address = dev_amm_pk.address().expect("address");
    let addr = dev_amm_rdg_address.render_string().expect("");

    println!("dev amm address: {addr}");

    if let Some(w) = std::env::var("REDGOLD_TEST_WORDS").ok() {
        let w = WordsPass::new(w, None);
        let path = "m/84'/0'/0'/0/0";
        let pk = w.public_at(path.to_string()).expect("private key");
        let privk = w.private_at(path.to_string()).expect("private key");
        let keypair = w.keypair_at(path.to_string()).expect("private key");

        let rdg_address = pk.address().expect("");
        println!("pk: {}", rdg_address.render_string().expect(""));

        let client = NodeConfig::dev_default().await.api_client();
        // client.faucet(&rdg_address).await.expect("faucet");
        let result = client.query_address(vec![rdg_address]).await.expect("").as_error().expect("");
        let utxos = result.query_addresses_response.safe_get_msg("missing query_addresses_response").expect("")
            .utxo_entries.clone();

        let amount = CurrencyAmount::from_fractional(0.01).expect("");
        let tb = TransactionBuilder::new()
            .with_utxos(&utxos).expect("utxos")
            .with_output(&dev_amm_rdg_address, &amount)
            .with_last_output_withdrawal_swap()
            .build().expect("build")
            .sign(&keypair).expect("sign");

        let res = client.send_transaction(&tb, true).await.expect("send").json_or();

        println!("send: {}", res);

        // let res = NodeConfig::dev_default().await.api_client().faucet(&rdg_address).await.expect("faucet");

        // println!("faucet: {}", res.json_or());
        // let mut w =
        //     SingleKeyBitcoinWallet::new_wallet(pk, NetworkEnvironment::Dev, true)
        //         .expect("w");
        // let a = w.address().expect("a");
        // // tb1qrxdzt6v9yuu567j52cmla4v9kler3wzj9swxy9
        // println!("wallet address: {a}");
        // let b = w.get_wallet_balance().expect("balance");
        // println!("wallet balance: {b}");
        //
        // let res = w.send_local(dev_amm_address, 2222, privk).expect("send");
        // println!("txid: {res}");
    }
}

#[ignore]
#[test]
pub fn dev_balance_check() {

    let dev_amm_address = "tb1qyxzxhpdkfdd9f2tpaxehq7hc4522f343tzgvt2".to_string();

    let pk_hex = "03879516077881c5be714024099c16974910d48b691c94c1824fad9635c17f3c37";
    let pk = PublicKey::from_hex(pk_hex).expect("pk");

    let addr = pk.address().expect("address").render_string().expect("");

    println!("address: {addr}");

    let mut w =
        SingleKeyBitcoinWallet::new_wallet(pk, NetworkEnvironment::Dev, true).expect("w");

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
