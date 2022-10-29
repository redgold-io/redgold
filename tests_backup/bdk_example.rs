use bdk::blockchain::{noop_progress, ElectrumBlockchain};
use bdk::database::MemoryDatabase;
use bdk::{FeeRate, TxBuilder, Wallet};

use bdk::electrum_client::Client;
use bdk::wallet::coin_selection::DefaultCoinSelectionAlgorithm;
use bdk::wallet::tx_builder::CreateTx;
use bdk::wallet::AddressIndex::New;

use bitcoin::consensus::serialize;

#[test]
fn example() -> Result<(), bdk::Error> {
    let client = Client::new("ssl://electrum.blockstream.info:60002")?;
    let client = ElectrumBlockchain::from(client);
    let database = MemoryDatabase::default();
    let wallet: Wallet<ElectrumBlockchain, MemoryDatabase> = Wallet::new(
        "wpkh([c258d2e4/84h/1h/0h]tpubDDYkZojQFQjht8Tm4jsS3iuEmKjTiEGjG6KnuFNKKJb5A6ZUCUZKdvLdSDWofKi4ToRCwb9poe1XdqfUnP4jaJjCB2Zwv11ZLgSbnZSNecE/0/*)",
        Some("wpkh([c258d2e4/84h/1h/0h]tpubDDYkZojQFQjht8Tm4jsS3iuEmKjTiEGjG6KnuFNKKJb5A6ZUCUZKdvLdSDWofKi4ToRCwb9poe1XdqfUnP4jaJjCB2Zwv11ZLgSbnZSNecE/1/*)"),
        bdk::bitcoin::Network::Testnet,
        database,
        client
    )?;

    wallet.sync(noop_progress(), None)?;

    let send_to = wallet.get_address(New)?;
    let (psbt, details) = {
        let mut builder: TxBuilder<
            ElectrumBlockchain,
            MemoryDatabase,
            DefaultCoinSelectionAlgorithm,
            CreateTx,
        > = wallet.build_tx();
        builder
            .add_recipient(send_to.script_pubkey(), 50_000)
            .enable_rbf()
            .do_not_spend_change()
            .fee_rate(FeeRate::from_sat_per_vb(5.0));
        builder.finish()?
    };

    println!("Transaction details: {:#?}", details);
    println!("psbt: {:#?}", psbt);
    // println!("Unsigned PSBT: {}", base64::encode(&serialize(&psbt)));

    Ok(())
}
