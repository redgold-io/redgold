
/*
balance: Balance { immature: 0, trusted_pending: 0, untrusted_pending: 0, confirmed: 6817 }
Source address: tb1q0287j37tntffkndch8fj38s2f994xk06rlr4w4
Send to address: tb1q68rhft47r5jwq5832k9urtypggpvzyh5z9c9gn
txid: 8080a06c0671d1492a24ef60fc1771cbba44cc5387dd754e434de3df4f8e9e5c
num signable hashes: 1
signable 0: a19abb028d61f5add0fbb033bbbe22677f9ab658e648b95ec84eb93edf5d81c4
finalized: true
test integrations::bitcoin::bdk_example::balance_test ... ok

 */
use std::path::PathBuf;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{NetworkEnvironment, PublicKey};
use crate::btc::btc_wallet::SingleKeyBitcoinWallet;
use crate::TestConstants;
use crate::util::keys::ToPublicKeyFromLib;
use crate::util::mnemonic_support::{test_pkey_hex, test_pubk};

#[ignore]
#[tokio::test]
async fn tx_debug() {
    // MnemonicWords::from_mnemonic_words()
    let _pkey = test_pkey_hex().expect("");
    let public = test_pubk().expect("");
    println!("Public key rg address {}", public.address().expect("").render_string().expect(""));
    let w = SingleKeyBitcoinWallet
    ::new_wallet(public, NetworkEnvironment::Test, true).expect("worx");
    let balance = w.get_wallet_balance().expect("");
    println!("balance: {:?}", balance);
    println!("address: {:?}", w.address().expect(""));
    // w.send_local("tb1qaq8de62av8xkcnwfrgjmvatsl56hmpc4q6m3uz".to_string(), 2500, pkey).expect("");
    // w.send_local("tb1q0287j37tntffkndch8fj38s2f994xk06rlr4w4".to_string(), 3500, pkey).expect("");
    // let txid = w.transaction_details.expect("d").txid.to_string();
    // println!("txid: {}", txid);
    // 2485227b319650fcd689009ca8b5fb2a02e556098f7c568e832ae72ac07ab8e8
}


// #[ignore]
// #[tokio::test]
// async fn balance_test2() {
//     let mut w = SingleKeyBitcoinWallet
//     ::new_wallet(PublicKey::from_hex_direct("028215a7bdab82791763e79148b4784cc7474f0969f23e44fea65d066602dea585").expect(""), NetworkEnvironment::Test, true).expect("worx");
//     let balance = w.get_wallet_balance().expect("");



//     println!("balance: {:?}", balance);
//     println!("address: {:?}", w.address().expect(""));
//     let txs = w.get_sourced_tx().expect("");
//     for t in txs {
//         println!("tx: {}", t.json_or());
//     }
//     let (_, kp) = dev_ci_kp().expect("");
//     let dest = kp.public_key().to_bitcoin_address(&NetworkEnvironment::Dev).expect("");
//     let tx = w.create_transaction(Some(kp.public_key()), None, 2200).expect("");
//     let psbt = w.psbt.expect("psbt");
//     let txb = psbt.clone().extract_tx();
//     println!("txb: {:?}", txb);
//     for o in txb.output {
//         println!("o: {:?}", o);

//     }


// }

#[ignore]
#[tokio::test]
async fn balance_test() {
    let tc = TestConstants::new();
    let _kp = tc.key_pair();
    // let pk = kp.public_key.to_struct_public_key();
    // let balance = get_balance(pk).expect("");
    // Source address: tb1q0287j37tntffkndch8fj38s2f994xk06rlr4w4
    // Send to address: tb1q68rhft47r5jwq5832k9urtypggpvzyh5z9c9gn
    let w = SingleKeyBitcoinWallet
    ::new_wallet(tc.public.to_struct_public_key(), NetworkEnvironment::Test, true).expect("worx");
    let balance = w.get_wallet_balance().expect("");
    println!("balance: {:?}", balance);
    println!("address: {:?}", w.address().expect(""));
    // w.get_source_addresses();
    let w2 = SingleKeyBitcoinWallet
    ::new_wallet(tc.public2.to_struct_public_key(), NetworkEnvironment::Test, true).expect("worx");
    let balance = w2.get_wallet_balance().expect("");
    println!("balance2: {:?}", balance);
    println!("address2: {:?}", w2.address().expect(""));
    // println!("{:?}", w2.get_sourced_tx().expect(""));


    // w.create_transaction(tc.public2.to_struct_public_key(), 3500).expect("");
    // let d = w.transaction_details.clone().expect("d");
    // println!("txid: {:?}", d.txid);
    // let signables = w.signable_hashes().expect("");
    // println!("num signable hashes: {:?}", signables.len());
    // for (i, (hash, sighashtype)) in signables.iter().enumerate() {
    //     println!("signable {}: {}", i, hex::encode(hash));
    //     let prf = Proof::from_keypair(hash, tc.key_pair());
    //     w.affix_input_signature(i, &prf, sighashtype);
    // }
    // let finalized = w.sign().expect("sign");
    // println!("finalized: {:?}", finalized);

    // w.broadcast_tx().expect("broadcast");
    // let txid = w.broadcast_tx().expect("txid");
    // println!("txid: {:?}", txid);
}

// // https://bitcoindevkit.org/blog/2021/12/first-bdk-taproot-tx-look-at-the-code-part-2/
// // https://github.com/bitcoin/bitcoin/blob/master/doc/descriptors.md


#[ignore]
#[tokio::test]
async fn balance_test_mn() {
    let mut w = SingleKeyBitcoinWallet
    ::new_wallet_db_backed(
        PublicKey::from_hex("0a230a210220f12e974037da99be8152333d4b72fc06c9041fbd39ac6b37fb6f65e3057c39")
            .expect(""), NetworkEnvironment::Main, true, PathBuf::from("testdb"),
        Some("ssl://fulcrum.sethforprivacy.com:50002".to_string()),
        None
    ).expect("worx");
    let balance = w.get_wallet_balance().expect("");


    println!("balance: {:?}", balance);
    println!("address: {:?}", w.address().expect(""));
    let txs = w.get_all_tx().expect("");
    for t in txs {
        println!("tx: {}", t.json_or());
    }


}
