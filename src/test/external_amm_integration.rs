use redgold_keys::util::btc_wallet::SingleKeyBitcoinWallet;
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_schema::structs::{NetworkEnvironment, PublicKey};

#[ignore]
pub fn load_ci_kp() {

    let dev_amm_address = "tb1qyxzxhpdkfdd9f2tpaxehq7hc4522f343tzgvt2".to_string();

    if let Some(w) = std::env::var("REDGOLD_TEST_WORDS").ok() {
        let w = WordsPass::new(w, None);
        let path = "m/84'/0'/0'/0/0";
        let pk = w.public_at(path.to_string()).expect("private key");
        let privk = w.private_at(path.to_string()).expect("private key");
        let mut w =
            SingleKeyBitcoinWallet::new_wallet(pk, NetworkEnvironment::Dev, true)
                .expect("w");
        let a = w.address().expect("a");
        // tb1qrxdzt6v9yuu567j52cmla4v9kler3wzj9swxy9
        println!("wallet address: {a}");
        let b = w.get_wallet_balance().expect("balance");
        println!("wallet balance: {b}");

        let res = w.send_local(dev_amm_address, 3500, privk).expect("send");
        println!("txid: {res}");
    }
}

#[ignore]
pub fn dev_balance_check() {

    let dev_amm_address = "tb1qyxzxhpdkfdd9f2tpaxehq7hc4522f343tzgvt2".to_string();

    let pk_hex = "03879516077881c5be714024099c16974910d48b691c94c1824fad9635c17f3c37";
    let pk = PublicKey::from_hex(pk_hex).expect("pk");

    let mut w =
        SingleKeyBitcoinWallet::new_wallet(pk, NetworkEnvironment::Dev, true).expect("w");
    let a = w.address().expect("a");
    println!("wallet address: {a}");
    let b = w.get_wallet_balance().expect("balance");
    println!("wallet balance: {b}");
    let c = b.confirmed;
    let t = b.get_total();
    let sp = b.get_spendable();
    println!("wallet confirmed: {c} total {t} spendable {sp}");
}
