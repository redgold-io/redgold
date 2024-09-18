use redgold::core::transact::tx_builder_supports::{TransactionBuilder, TransactionBuilderSupport};
use redgold::e2e::tx_submit::TransactionSubmitter;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_keys::eth::example::dev_ci_kp;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_schema::structs::{CurrencyAmount, ErrorInfo, NetworkEnvironment};
use redgold::core::transact::tx_broadcast_support::TxBroadcastSupport;

#[tokio::test]
async fn release_it() -> Result<(), ErrorInfo> {

    // util::init_logger();;
    println!("Local E2E Release test from inside test");

    let nc = NodeConfig::default_env(NetworkEnvironment::parse(std::env::var("REDGOLD_NETWORK").unwrap())).await;

    let (privk, kp) = dev_ci_kp().unwrap();
    let client = nc.api_rg_client();
    let party_key = client.active_party_key().await.unwrap();

    println!("Awaiting docker image turn");
    tokio::time::sleep(std::time::Duration::from_secs(60*30)).await;

    let exe_checksum_start = client.executable_checksum().await.unwrap();
    let mut retries = 0;
    let max_attempts = 100;
    let mut success = false;

    while retries < max_attempts {
        if let Ok(new_exe_checksum) = client.executable_checksum().await {
            if new_exe_checksum != exe_checksum_start {
                success = true;
                break;
            }
        }
        retries += 1;
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }

    if !success {
        panic!("Executable checksum did not change after {} attempts", max_attempts);
    }

    // Await other nodes online
    tokio::time::sleep(std::time::Duration::from_secs(60*5)).await;

    let mut txb = TransactionBuilder::new(&nc);
    let res = txb.with_input_address(&kp.address_typed())
        .with_auto_utxos().await.unwrap()
        .with_output(&party_key.address().unwrap(), &CurrencyAmount::from_fractional(0.01).unwrap())
        .build()
        .unwrap()
        .sign(&kp)
        .unwrap()
        .broadcast()
        .await
        .unwrap();
    res.at_least_n(2).expect("at least 2 nodes to respond");

    Ok(())

}