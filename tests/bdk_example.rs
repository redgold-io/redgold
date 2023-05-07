use bdk::blockchain::{noop_progress, ElectrumBlockchain};
use bdk::database::MemoryDatabase;
use bdk::{FeeRate, SignOptions, TxBuilder, Wallet};

use bdk::electrum_client::Client;
use bdk::wallet::coin_selection::DefaultCoinSelectionAlgorithm;
use bdk::wallet::tx_builder::CreateTx;
use bdk::wallet::AddressIndex::New;

use bitcoin::consensus::serialize;
use redgold::util::cli::commands::send;
use redgold_schema::{KeyPair, TestConstants};

#[test]
fn schnorr_test() {
    let tc = TestConstants::new();
    let kp = tc.key_pair();

}

// https://bitcoindevkit.org/blog/2021/12/first-bdk-taproot-tx-look-at-the-code-part-2/
// https://github.com/bitcoin/bitcoin/blob/master/doc/descriptors.md
#[test]
fn example()  {

    // // bdk = { version = "0.24.0", features = [ "hardware-signer", ] }
    // let client = Client::new("ssl://electrum.blockstream.info:60002")?;
    // let client = ElectrumBlockchain::from(client);
    // let database = MemoryDatabase::default();
    // let wallet: Wallet<ElectrumBlockchain, MemoryDatabase> = Wallet::new(
    //     "wpkh([c258d2e4/84h/1h/0h]tpubDDYkZojQFQjht8Tm4jsS3iuEmKjTiEGjG6KnuFNKKJb5A6ZUCUZKdvLdSDWofKi4ToRCwb9poe1XdqfUnP4jaJjCB2Zwv11ZLgSbnZSNecE/0/*)",
    //     Some("wpkh([c258d2e4/84h/1h/0h]tpubDDYkZojQFQjht8Tm4jsS3iuEmKjTiEGjG6KnuFNKKJb5A6ZUCUZKdvLdSDWofKi4ToRCwb9poe1XdqfUnP4jaJjCB2Zwv11ZLgSbnZSNecE/1/*)"),
    //     bdk::bitcoin::Network::Testnet,
    //     database,
    //     client
    // )?;
    //
    // wallet.sync(noop_progress(), None)?;
    //
    // let bal = wallet.get_balance();
    //
    // println!("Balance: {:?}", bal);
    //
    //
    // let send_to = wallet.get_address(New)?;
    // let addr_str = send_to.to_string();
    //
    // println!("send_to: {}", send_to);
    //
    // // https://bitcoinfaucet.uo1.net/send.php
    // // return address tb1q4280xax2lt0u5a5s9hd4easuvzalm8v9ege9ge
    // let ( mut psbt, details) = {
    //     let mut builder: TxBuilder<
    //         ElectrumBlockchain,
    //         MemoryDatabase,
    //         DefaultCoinSelectionAlgorithm,
    //         CreateTx,
    //     > = wallet.build_tx();
    //     builder
    //         .add_recipient(send_to.script_pubkey(), 50_000)
    //         .enable_rbf()
    //         .do_not_spend_change()
    //         .fee_rate(FeeRate::from_sat_per_vb(5.0));
    //     builder.finish()?
    // };
    //
    // wallet.sign(&mut psbt, SignOptions::default())?;
    // println!("Transaction details: {:#?}", details);
    // println!("psbt: {:#?}", psbt);
    // // println!("Signed PSBT: {}", base64::encode(&serialize(&psbt)));
    // // println!("Unsigned PSBT: {}", base64::encode(&serialize(&psbt)));
    //
    // Ok(())
}
