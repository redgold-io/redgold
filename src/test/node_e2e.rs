use crate::api::client::public_client::PublicClient;
use redgold_common::client::http::RgHttpClient;
use crate::api::control_api::ControlClient;
use crate::core::relay::Relay;
use crate::e2e::tx_submit::TransactionSubmitter;
use crate::node::Node;
use crate::node_config::ToTransactionBuilder;
use crate::observability::metrics_registry;
use crate::party::stake_event_stream::StakeMethods;
use crate::test::harness::amm_harness::PartyTestHarness;
use crate::test::local_test_context::{LocalNodes, LocalTestNodeContext};
use crate::util;
use crate::util::runtimes::{big_thread, build_simple_runtime};
use itertools::Itertools;
use redgold_keys::address_external::ToEthereumAddress;
use redgold_keys::proof_support::ProofSupport;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::util::mnemonic_support::MnemonicSupport;
use redgold_keys::word_pass_support::WordsPassNodeConfig;
use redgold_keys::{KeyPair, TestConstants};
use redgold_rpc_integ::eth::eth_wallet::EthWalletWrapper;
use redgold_rpc_integ::eth::historical_client::EthHistoricalClient;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::keys::words_pass::WordsPass;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::party::party_events::PartyEvents;
use redgold_schema::proto_serde::{ProtoHashable, ProtoSerde};
use redgold_schema::structs::{Address, ControlMultipartyKeygenResponse, ControlMultipartySigningRequest, CurrencyAmount, ErrorInfo, Hash, InitiateMultipartySigningRequest, NetworkEnvironment, Proof, PublicKey, Seed, SupportedCurrency, TestContractInternalState, Transaction, UtxoEntry};
use redgold_schema::tx::tx_builder::TransactionBuilder;
use redgold_schema::{bytes_data, structs, ErrorInfoContext, RgResult, SafeOption};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tracing::info;
use redgold_rpc_integ::examples::example::dev_ci_kp;

/// Main entry point for end to end testing.
/// Workaround used to avoid config related overflow caused by clap et al.
// #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
// #[tokio::test]
#[test]
fn e2e() {
// async fn e2e() {
    redgold_common::log::init_logger_once();

    let result = big_thread().spawn(|| {
        let runtime = build_simple_runtime(num_cpus::get(), "config");
        let ret = runtime.block_on(e2e_async(false));
        runtime.shutdown_background();
        ret
    }).unwrap().join().unwrap();

    // let result = .await.log_error();
    // Allow time to catch main service error
    // tokio::time::sleep(Duration::from_secs(2)).await;
    result.expect("e2e");
}


async fn e2e_async(contract_tests: bool) -> Result<(), ErrorInfo> {
    let _tc = TestConstants::new();

    let mut local_nodes = LocalNodes::new(None).await;

    let start_node: LocalTestNodeContext = local_nodes.start().clone();
    let config = start_node.node.relay.node_config.clone();
    let client = start_node.public_client.clone();

    let vec = start_node.node.relay.ds.utxo.utxo_all_debug().await.expect("utxo all debug");
    assert!(vec.len() > 0);

    let (_, spend_utxos) = Node::genesis_from(start_node.node.relay.node_config.clone());

    let submit = TransactionSubmitter::default(client.clone(),
                                               // runtime.clone(),
                                               spend_utxos,
        &start_node.node.relay.node_config
    );

    // single_node_tests(&mut local_nodes, &submit).await;

    local_nodes.add_node(
        // runtime.clone()
    ).await;

    // two_node_tests(&mut local_nodes, &submit).await;
    // two_node_keygen_test(&mut local_nodes, &client1, &submit).await?;

    // three nodes
    local_nodes.add_node().await;

    // info!("Three node keygen test");
    // three_node_keygen_tests(&mut local_nodes, start_node.control_client.clone(), &submit).await?;
    // info!("Three node keygen test passed");

    let kp = WordsPass::test_words().keypair_at_change(0).unwrap();
    // let kp = dev_ci_kp().expect("kp").1;

    tokio::time::sleep(Duration::from_secs(40)).await;


    let mut config2 = config.clone();
    let string = client.client_wrapper().url();
    // info!("setting test harness to {} ", string.clone());
    // info!("active party key {}", client.client_wrapper().active_party_key().await.expect("works").json_or());
    config2.load_balancer_url = string;
    let vec = local_nodes.ext.clone();

    let mut party_harness = PartyTestHarness::from(
        &config2, vec![vec], Some(client.client_wrapper()), vec![]).await;

    let address = party_harness.self_rdg_address();
    submit.send_to(&address).await.expect("works");
    // submit.send_to(&address).await.expect("works");
    // submit.send_to(&address).await.expect("works");
    //
    // let b = client.balance(address).await.expect("works");
    // info!("Balance: {}", b.json_or());
    party_harness.run_test().await.expect("works");

    // party_harness.run_portfolio_test().await;
    //
    // // Manual test uses up funds.

    // manual_eth_mp_signing_test(client1, keygen2, &mp_eth_addr, &environment).await;
    // TODO: AMM tests

    // Not triggering in tests, confirmation time is too long for BTC for a proper test, need to wait for
    // ETH support.
    // let ds = start_node.node.relay.ds.clone();
    //
    // let mut loaded = false;
    // for _ in 0..10 {
    //     let test_load = ds.config_store.get_json::<DepositWatcherConfig>("deposit_watcher_config").await;
    //     if let Ok(Some(t)) = test_load {
    //         info!("Deposit watcher config: {}", t.json_or());
    //         loaded = true;
    //         break;
    //     }
    //     tokio::time::sleep(Duration::from_secs(2)).await;
    // }
    // assert!(loaded);

    // Eth staking tests.
    // ignore for now, too flakey.
    // if false {
    // eth_amm_e2e(start_node, relay_start, &submit).await.expect("works");

    info!("Test passed");

    std::mem::forget(local_nodes);
    std::mem::forget(submit);
    Ok(())
}

async fn two_node_keygen_test(local_nodes: &mut LocalNodes, client1: &ControlClient, submit: &TransactionSubmitter) -> Result<(), ErrorInfo> {
    // let keygen1 = client1.multiparty_keygen(None).await.log_error()?;

    // tokio::time::sleep(Duration::from_secs(10)).await;


    // let signing_data = Hash::from_string_calculate("hey");
    // let _result = do_signing(keygen1.clone(), signing_data.clone(), client1.clone()).await;

    tracing::info!("After MP test");

    submit.with_faucet().await.unwrap().submit_transaction_response.expect("").at_least_n(2).unwrap();

    local_nodes.verify_data_equivalent().await;
    Ok(())
}

async fn two_node_tests(local_nodes: &mut LocalNodes, submit: &TransactionSubmitter) {
    local_nodes.verify_data_equivalent().await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    let after_2_nodes = submit.submit().await.expect("submit");
    after_2_nodes.at_least_n(2).unwrap();

    local_nodes.verify_peers().await.expect("verify peers");
}

async fn single_node_tests(local_nodes: &mut LocalNodes, submit: &TransactionSubmitter) {
    submit.submit().await.expect("submit");
    //
    // if contract_tests {
    //     let res = submit.submit_test_contract().await.expect("submit test contract");
    //     let ct = res.transaction.expect("tx");
    //     let contract_address = ct.first_output_address_non_input_or_fee().expect("cont");
    //     let _o = ct.outputs.get(0).expect("O");
    //     let state = client.client_wrapper().contract_state(&contract_address).await.expect("res");
    //     let state_json = TestContractInternalState::proto_deserialize(state.state.clone().expect("").value).expect("").json_or();
    //     info!("First contract state marker: {} {}", state.json_or(), state_json);
    //
    //     submit.submit_test_contract_call(&contract_address ).await.expect("worx");
    //     let state = client.client_wrapper().contract_state(&contract_address).await.expect("res");
    //     let state_json = TestContractInternalState::proto_deserialize(state.state.clone().expect("").value).expect("").json_or();
    //     info!("Second contract state marker: {} {}", state.json_or(), state_json);
    //     return Ok(());
    // }
    //
    //
    // // let utxos = ds.query_time_utxo(0, util::current_time_millis())
    // //     .unwrap();
    // // info!("Num utxos after first submit {:?}", utxos.len());
    //
    //
    // Exception bad access on this on the json decoding? wtf?
    let _ = submit.with_faucet().await.expect("faucet");
    // info!("Faucet response: {}", faucet_res.json_pretty_or());

    submit.submit().await.expect("submit 2");
    //
    // // info!("Num utxos after second submit {:?}", utxos.len());
    //
    submit.submit_duplicate().await;
    //
    // // info!("Num utxos after duplicate submit {:?}", utxos.len());
    //
    // // show_balances();
    // // // shouldn't response metadata be not an option??
    //
    for _ in 0..1 {
        // TODO Flaky failure observed once? Why?
        submit.submit_double_spend(None).await;
    }
    //
    // // TODO: Submit invalid UTXO
    submit.submit_invalid_signature().await;
    submit.submit_used_mismatched_utxo().await;
    submit.submit_used_utxo().await;

    local_nodes.verify_data_equivalent().await;
}
