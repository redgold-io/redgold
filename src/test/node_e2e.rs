use crate::api::client::public_client::PublicClient;
use crate::api::client::rest::RgHttpClient;
use crate::api::control_api::ControlClient;
use crate::core::relay::Relay;
use crate::e2e::tx_submit::TransactionSubmitter;
use crate::multiparty_gg20::initiate_mp::default_room_id_signing;
use crate::node::Node;
use crate::node_config::ToTransactionBuilder;
use crate::observability::metrics_registry;
use crate::party::stake_event_stream::StakeMethods;
use crate::test::external_amm_integration::dev_ci_kp;
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
use rocket::yansi::Paint;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tracing::info;


/// Main entry point for end to end testing.
/// Workaround used to avoid config related overflow caused by clap et al.
// #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
// #[tokio::test]
#[test]
fn e2e() {
// async fn e2e() {
    util::init_logger_once();

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

    info!("Three node keygen test");
    three_node_keygen_tests(&mut local_nodes, start_node.control_client.clone(), &submit).await?;
    info!("Three node keygen test passed");

    let kp = WordsPass::test_words().keypair_at_change(0).unwrap();
    // let kp = dev_ci_kp().expect("kp").1;

    tokio::time::sleep(Duration::from_secs(40)).await;


    let mut config2 = config.clone();
    let string = client.client_wrapper().url();
    info!("setting test harness to {} ", string.clone());
    info!("active party key {}", client.client_wrapper().active_party_key().await.expect("works").json_or());
    config2.load_balancer_url = string;
    let vec = local_nodes.ext.clone();

    let party_harness = PartyTestHarness::from(
        &config2, kp, vec![vec], Some(client.client_wrapper()), vec![]).await;

    let address = party_harness.self_rdg_address();
    submit.send_to(&address).await.expect("works");
    // submit.send_to(&address).await.expect("works");
    // submit.send_to(&address).await.expect("works");
    //
    // let b = client.balance(address).await.expect("works");
    // info!("Balance: {}", b.json_or());
    // party_harness.run_test().await.expect("works");

    party_harness.run_portfolio_test().await;
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

async fn three_node_keygen_tests(local_nodes: &mut LocalNodes, client1: ControlClient, submit: &TransactionSubmitter) -> Result<(), ErrorInfo> {
    local_nodes.verify_data_equivalent().await;
    local_nodes.verify_peers().await?;
    //
    // This works but is really flaky for some reason?
    // submit.with_faucet().await.unwrap().submit_transaction_response.expect("").at_least_n(3).unwrap();

    // submit.submit().await?.at_least_n(3).unwrap();

    let signing_data = Hash::from_string_calculate("hey");

    let first_two = local_nodes.nodes.iter().take(2).map(|n| n.node.relay.node_config.public_key())
        .collect_vec();

    let first_three = local_nodes.nodes.iter().take(3).map(|n| n.node.relay.node_config.public_key())
        .collect_vec();

    let keygen2 = client1.multiparty_keygen(None).await.log_error()?;
    // info!("Keygen 2 response {}", keygen2.json_or());
    // required to get the public key
    info!("Starting first signing round with ALL");
    let res = do_signing(keygen2.clone(), signing_data.clone(), client1.clone(), first_three, true).await;
    res.verify(&signing_data.clone()).expect("verify");
    // info!("Starting second signing round with ALL-1");
    // let res = do_signing(keygen2.clone(), signing_data.clone(), client1.clone(), first_two, false).await;
    // res.verify(&signing_data.clone()).expect("verify");


    // let public = res.public_key.expect("public key");
    // let mp_eth_addr = public.to_ethereum_address().expect("eth address");
    //
    // let environment = NetworkEnvironment::Dev;
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
//
// async fn manual_eth_mp_signing_test(client1: ControlClient, keygen2: ControlMultipartyKeygenResponse, mp_eth_addr: &String, environment: &NetworkEnvironment) {
// // Ignore this part for now
//     let h = EthHistoricalClient::new(&environment).expect("works").expect("works");
//     let string_addr = "0xA729F9430fc31Cda6173A0e81B55bBC92426f759".to_string();
//     let txs = h.get_all_tx(&string_addr, None).await.expect("works");
//     println!("txs: {}", txs.json_or());
//     let tx_head = txs.get(0).expect("tx");
//     let _other_address = tx_head.other_address.clone();
//
//     // Load using the faucet KP, but send to the multiparty address
//     let (dev_secret, dev_kp) = dev_ci_kp().expect("works");
//     let eth = EthWalletWrapper::new(&dev_secret, &environment).expect("works");
//     let dev_faucet_rx_addr = dev_kp.public_key().to_ethereum_address().expect("works");
//     let fee = "0.000108594791676".to_string();
//     let fee_value = EthHistoricalClient::translate_float_value(&fee.to_string()).expect("works") as u64;
//     let amount = fee_value * 6;
//     let _tx = eth.send(&mp_eth_addr, amount).await.expect("works");
//
//     tokio::time::sleep(Duration::from_secs(20)).await;
//
//     let mut tx = eth.create_transaction(&mp_eth_addr, &dev_faucet_rx_addr, fee_value * 3).await.expect("works");
//     let data = EthWalletWrapper::signing_data(&tx).expect("works");
//     let h = Hash::new_from_proto(data).expect("works");
//     let res = do_signing(keygen2.clone(), h.clone(), client1.clone()).await;
//     let sig = res.signature.expect("sig");
//     let raw = EthWalletWrapper::process_signature(sig, &mut tx).expect("works");
//     eth.broadcast_tx(raw).await.expect("works");
// }

async fn eth_amm_e2e(start_node: LocalTestNodeContext, relay_start: Relay, submit: &TransactionSubmitter) -> RgResult<()> {
    if let Some((secret, kp)) = dev_ci_kp() {


        // Await party formation
        let mut retries = 0;
        loop {
            if let Some((party_public_key, pid)) = relay_start.external_network_shared_data.clone_read().await
                .into_iter().filter(|(k, v)| {
                v.self_initiated_not_debug()
            }).next() {
                info!("Party formation pk: {}", party_public_key.json_or());
                let all_in = pid.party_info.initiate.expect("init").identifier.expect("id").party_keys.len() == 3;
                if all_in {
                    break;
                } else {
                    panic!("Not all parties in formation");
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            retries += 1;
            if retries > 60 {
                panic!("Party formation not completed in expected time");
            }
        }

        // TODO: Mock request for API to get pool information.
        let (party_public_key, pid) = relay_start.external_network_shared_data.clone_read().await
            .into_iter().filter(|(k, v)| {
            v.self_initiated_not_debug()
        }).next().expect("works");
        let config = start_node.node.relay.node_config.clone();

        let party_rdg_address = party_public_key.address().expect("works");
        let dev_ci_rdg_address = kp.address_typed();

        // First send some funds to pay for fees.
        let utxos = submit.send_to_return_utxos(&dev_ci_rdg_address).await.expect("works");
        let amt = utxos.iter().map(|u| u.output.as_ref().expect("works").amount_i64()).sum::<i64>();
        let internal_stake_amount = CurrencyAmount::from_rdg(amt - (amt / 10));
        assert!(!utxos.is_empty());


        // Then send the internal RDG stake
        let signed_internal_stake_tx = config.tx_builder().with_utxos(&utxos).expect("works")
            .with_internal_stake_usd_bounds(
                None, None, &dev_ci_rdg_address, &party_rdg_address, &internal_stake_amount,
            ).build().expect("works").sign(&kp).expect("works");

        info!("Sending internal stake");

        submit.send_tx(&signed_internal_stake_tx).await.expect("works");
        info!("Finished internal stake");

        let txs = relay_start.ds.transaction_store.get_all_tx_for_address(&party_rdg_address, 1e9 as i64, 0).await.expect("works");
        txs.iter().filter(|tx| tx.hash_or() == signed_internal_stake_tx.hash_or()).next().expect("works");


        let all_utxo_internal_stake = signed_internal_stake_tx.to_utxo_address(&dev_ci_rdg_address);
        let stake_internal_utxo_for_withdrawal = all_utxo_internal_stake.iter()
            .filter(|u| u.output.as_ref().expect("works").stake_request().is_some())
            .cloned()
            .next()
            .expect("works");

        let mut retries = 0;
        loop {
            if let Some((_, pid)) = relay_start.external_network_shared_data.clone_read().await
                .into_iter().filter(|(k, v)| {
                v.self_initiated_not_debug()
            }).next() {
                if let Some(pe) = pid.party_events {
                    if pe.internal_staking_events.len() > 0 {
                        info!("Found internal stake event");
                        break;
                    }
                }
            }
            info!("Awaiting internal stake event");
            tokio::time::sleep(Duration::from_secs(5)).await;
            retries += 1;
            if retries > 8 {
                panic!("No internal stake event found");
            }
        }


        // Then send the external ETH stake registration request

        info!("Getting utxos for external stake test");
        let test_tx = submit.send_to(&dev_ci_rdg_address).await.expect("works").transaction.expect("works");
        info!("Got utxos for external stake test");
        // info!("Test tx: {}", test_tx.json_or());
        // info!("Test tx time: {}", test_tx.time().expect("time").to_string());
        let utxos_tx_external_stake = test_tx.to_utxo_address(&dev_ci_rdg_address);

        let dev_ci_eth_addr = kp.public_key().to_ethereum_address_typed().expect("works");
        let exact_eth_stake_amount = CurrencyAmount::stake_test_amount_typed();
        let party_fee_amount = CurrencyAmount::from_rdg(100000);

        let tx_stake = config.tx_builder().with_utxos(&utxos_tx_external_stake)?
            .with_external_stake_usd_bounds(
                None,
                None,
                &dev_ci_rdg_address,
                &dev_ci_eth_addr,
                &exact_eth_stake_amount,
                &party_rdg_address,
                &party_fee_amount
            ).build().expect("works").sign(&kp).expect("works");

        // info!("tx_stake tx time: {}", tx_stake.time().expect("time").to_string());
        // info!("tx_stake tx: {}", tx_stake.json_or());

        let res = submit.send_tx(&tx_stake).await.expect("works");

        // verify internal tx for external stake is present
        let mut retries = 0;
        loop {
            if let Some((_, pid)) = relay_start.external_network_shared_data.clone_read().await
                .into_iter().filter(|(k, v)| {
                v.self_initiated_not_debug()
            }).next() {
                if let Some(pe) = pid.party_events {
                    if pe.pending_external_staking_txs.len() > 0 {
                        info!("Found pending external stake event"); //: {}", pe.external_unfulfilled_staking_txs.json_or());
                        break;
                    }
                }
            }
            info!("Awaiting internal pending external stake event");
            tokio::time::sleep(Duration::from_secs(10)).await;
            retries += 1;
            if retries > 10 {
                panic!("No internal pending external stake event found");
            }
        }


        let stake_external_utxo_for_withdrawal = tx_stake.to_utxo_address(&dev_ci_rdg_address).iter()
            .filter(|u| u.output.as_ref().expect("works").stake_request().is_some())
            .cloned()
            .next()
            .expect("works");

        assert!(relay_start.ds.utxo.utxo_id_valid(
            stake_external_utxo_for_withdrawal.utxo_id.as_ref().expect("u")).await.expect("works"));
        // Then send the ETH stake
        let party_eth_address = party_public_key.to_ethereum_address_typed().expect("works");

        info!("Sending eth stake to party address");
        let eth = EthWalletWrapper::new(&secret, &config.network).expect("works");
        info!("Fee estimate {}", eth.get_fee_estimate().await.expect("works").json_or());
        info!("Fee fixed {}", CurrencyAmount::eth_fee_fixed_normal_testnet().json_or());
        let res = tokio::time::timeout(
            Duration::from_secs(120), eth.send(
                &party_eth_address, &exact_eth_stake_amount, None)
        ).await.expect("works").expect("works");
        info!("Eth txid: {}", res);
        let mut retries = 0;
        loop {
            info!("Sent eth stake to party address, awaiting receipt");

            let (ppk, pid) = relay_start.external_network_shared_data.clone_read().await
                .into_iter().filter(|(k, v)| {
                v.party_info.not_debug() && v.party_info.self_initiated()
            }).next().expect("works");
            if let Some(pev) = pid.party_events.as_ref() {
                let balance = pev.balance_map.get(&SupportedCurrency::Ethereum);
                if let Some(b) = balance {
                    info!("Party balance after eth stake: {}", b.json_or());
                    break;
                }
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
            retries += 1;
            if retries > 20 {
                panic!("Failed to receive ETH stake");
            }
        }

        let key = std::env::var("ETHERSCAN_API_KEY2").expect("works");
        let eth_h = EthHistoricalClient::new_from_key(&config.network, key).expect("works");
        // From here we need to wrap everything in a function, so that we can catch failures to withdraw this stake.
        //
        let maybe_err = proceed_swap_test_from_eth_send(
            &config,
            &party_rdg_address,
            &dev_ci_rdg_address,
            &dev_ci_eth_addr,
            &kp,
            &eth,
            &submit,
            &party_eth_address,
            &relay_start,
            &eth_h
        ).await.log_error();


        info!("Finished proceed_swap_test_from_eth_send");

        let new_utxos = submit.send_to_return_utxos(&dev_ci_rdg_address).await.expect("works");

        info!("Got UTXOs for final eth stake withdrawal");
        for u in new_utxos.iter() {
            assert!(relay_start.ds.utxo.utxo_id_valid(u.utxo_id.as_ref().expect("u")).await.expect("works"));
        };

        tokio::time::sleep(Duration::from_secs(20)).await;
        let eth_withdrawal = config.tx_builder().with_utxos(&new_utxos).expect("works")
            .with_stake_withdrawal(
                &dev_ci_eth_addr,
                &party_rdg_address,
                &party_fee_amount,
                &stake_external_utxo_for_withdrawal.utxo_id.unwrap()
            ).build().expect("works").sign(&kp).expect("works");

        info!("Sending eth withdrawal {}", eth_withdrawal.json_or());
        info!("Sending eth withdrawal hash {}", eth_withdrawal.hash_hex());

        let res = submit.send_tx(&eth_withdrawal).await.expect("works");
        let tx = res.transaction.expect("works");

        let mut retries = 0;
        let original_eth_balance = eth_h.get_balance_typed(&dev_ci_eth_addr).await?;
        let amount_orig = original_eth_balance.amount_i64_or();
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            info!("Awaiting receipt of ETH stake withdrawal");
            let new_balance = eth_h.get_balance_typed(&dev_ci_eth_addr).await?;
            let new_amount = new_balance.amount_i64_or();
            if new_amount > amount_orig {
                break;
            }
            retries += 1;
            if retries > 10 {
                return Err(ErrorInfo::error_info("Failed to receive ETH stake withdrawal"));
            }
        };


        let new_utxos = submit.send_to_return_utxos(&dev_ci_rdg_address).await.expect("works");

        info!("Preparing RDG withdrawal after ETH withdrawal success?");
        let rdg_withdrawal = config.tx_builder().with_utxos(&new_utxos).expect("works")
            .with_stake_withdrawal(
                &dev_ci_rdg_address,
                &party_rdg_address,
                &party_fee_amount,
                &stake_internal_utxo_for_withdrawal.utxo_id.unwrap()
            ).build().expect("works").sign(&kp).expect("works");

        let res = submit.send_tx(&rdg_withdrawal).await.expect("works");
        info!("Finished RDG withdrawal success");

        maybe_err.expect("works");
    };
    Ok(())
}

async fn proceed_swap_test_from_eth_send(
    config: &NodeConfig,
    party_rdg_address: &Address,
    dev_ci_rdg_address: &Address,
    dev_ci_eth_addr: &Address,
    kp: &KeyPair,
    eth: &EthWalletWrapper,
    submit: &TransactionSubmitter,
    party_eth_address: &Address,
    relay: &Relay,
    h: &EthHistoricalClient
) -> RgResult<()> {
    //
    // // This is sending RDG to receive ETH
    // info!("Getting UTXOs for test send rdg_receive_eth");
    // let utxos = submit.send_to_return_utxos(&dev_ci_rdg_address).await.expect("works");
    // let bal = utxos.iter().flat_map(|u| u.opt_amount()).map(|a| a.amount).sum::<i64>();
    // let amount = CurrencyAmount::from_rdg(bal - bal / 10);
    // let original_eth_balance = h.get_balance_typed(&dev_ci_eth_addr).await?;
    // let amount_orig = original_eth_balance.amount_i64_or();
    // let send_rdg_receive_eth = config.tx_builder()
    //     .with_utxos(&utxos).expect("works")
    //     .with_swap(
    //         &dev_ci_eth_addr,
    //         &amount,
    //         party_rdg_address,
    //     )?
    //     .build()?
    //     .sign(&kp).expect("works");
    // info!("Submitting send_rdg_receive_eth");
    //
    // let res = submit.send_tx(&send_rdg_receive_eth).await?;
    // let mut retries = 0;
    // loop {
    //     tokio::time::sleep(Duration::from_secs(10)).await;
    //     info!("Awaiting receipt of ETH swap after sending RDG");
    //     let new_balance = h.get_balance_typed(&dev_ci_eth_addr).await?;
    //     let new_amount = new_balance.amount_i64_or();
    //     if new_amount > amount_orig {
    //         break;
    //     }
    //     retries += 1;
    //     if retries > 10 {
    //         return Err(ErrorInfo::error_info("Failed to receive ETH swap"));
    //     }
    // };
    //
    // let rdg_balance = relay.ds.transaction_store
    //     .get_balance(&party_rdg_address).await?.ok_msg("works")?;

    info!("Submitting eth direct deposit to RDG swap");

    // TODO: Should we use SwapRequest to represent an external event? Not necessary for now
    // since this will just directly issue a swap.

    eth.send(
        &party_eth_address,
        &CurrencyAmount::test_send_amount_typed(),
        None
    ).await?;
    // let fee_amount_pool = CurrencyAmount::from_rdg(10000);

    let mut receive_addr = dev_ci_eth_addr.clone();
    receive_addr.set_currency(SupportedCurrency::Redgold);
    let rdg_eth_receive_addr_bal = relay.ds.transaction_store
        .get_balance(&receive_addr).await?.unwrap_or(0);

    let mut retries = 0;
    loop {
        tokio::time::sleep(Duration::from_secs(10)).await;
        info!("Awaiting receipt of RDG from swap after sending ETH");
        let new_bal = relay.ds.transaction_store
            .get_balance(&receive_addr).await?.unwrap_or(0);
        if new_bal > rdg_eth_receive_addr_bal {
            break;
        }
        retries += 1;
        if retries > 10 {
            return Err(ErrorInfo::error_info("Failed to receive RDG swap"));
        }
    };

    info!("Swap success of RDG from sent eth");

    Ok(())
}

#[ignore]
#[test]
fn env_var_parse_test() {

    println!("Env var test")

}

#[ignore]
#[tokio::test]
async fn data_store_test() {
    let nc = NodeConfig::from_test_id(&(100 as u16));
    let relay = Relay::new(nc.clone()).await;
    Node::prelim_setup(relay.clone()).await.expect("");
    let tx_0_hash = nc.peer_tx_fixed().hash_or();
    let hash_vec = tx_0_hash.vec();
    let mut txs = vec![];
    for i in 0..10 {
        let nci = NodeConfig::from_test_id(&(i + 200 as u16));
        let tx = nci.peer_tx_fixed();
        relay.write_transaction(&tx, 0, None, true).await.expect("");
        txs.push(tx.clone());
    }

    println!("original tx hash: {}", hex::encode(hash_vec.clone()));

    for tx in txs {
        let h = tx.hash_or();
        let v1 = hash_vec.clone();
        let v2 = h.vec();
        let xor_value: Vec<u8> = v1
            .iter()
            .zip(v2.iter())
            .map(|(&x1, &x2)| x1 ^ x2)
            .collect();
        let distance: u64 = xor_value.iter().map(|&byte| u64::from(byte)).sum();
        println!("hash distance {} xor_value: {} tx_hash {}", distance, hex::encode(xor_value.clone()), h.hex());
    }

    // let ds_ret = relay.ds.transaction_store.xor_transaction_order(&tx_0_hash).await.expect("");
    //
    // for (tx, xor_value) in ds_ret {
    //     println!("xor_value: {} tx_hash: {}", hex::encode(xor_value), hex::encode(tx));
    // }

}

async fn do_signing(
    party: ControlMultipartyKeygenResponse,
    signing_data: Hash,
    client: ControlClient,
    restrict_peers: Vec<PublicKey>,
    skip_party_key: bool
) -> Proof {

    let vec1 = signing_data.raw_bytes().expect("works");
    let vec = bytes_data(vec1.clone()).expect("");
    let mut signing_request = ControlMultipartySigningRequest::default();
    let mut init_signing = InitiateMultipartySigningRequest::default();
    let identifier = party.multiparty_identifier.expect("");
    init_signing.signing_room_id = default_room_id_signing(&identifier.room_id.clone().expect("rid")).ok();
    init_signing.data_to_sign = Some(vec);
    init_signing.identifier = Some(identifier.clone());
    init_signing.signing_party_keys = restrict_peers;
    init_signing.skip_party_key_lookup = Some(skip_party_key);
    signing_request.signing_request = Some(init_signing);
    let res =
        client.multiparty_signing(signing_request).await.log_error().unwrap();
    // println!("{:?}", res);
    // assert!(res.is_ok());
    let proof = res.proof.expect("prof");
    proof.verify(&signing_data).expect("verified");
    proof

}

#[ignore]
#[tokio::test]
async fn e2e_dbg() {
    e2e_async(true).await.expect("");
    // let runtime = build_runtime(8, "e2e");
    // runtime.block_on(e2e_async()).expect("e2e");
}


#[ignore]
#[tokio::test]
async fn debug_send() {

    let dest = "0xA13072d258b2dA7C825f1335F5aa5aA6a31E2829";

    let (dev_secret, dev_kp) = dev_ci_kp().expect("works");

    let eth = EthWalletWrapper::new(&dev_secret, &NetworkEnvironment::Dev).expect("works");

    let a = structs::Address::from_eth(&dest.to_string());
    let amt = CurrencyAmount::stake_test_amount_typed();

    assert!(PartyEvents::meets_minimum_stake_amount(&amt));
    // eth.send_tx_typed(&a, &amt).await.expect("works");
    let destination = Address::from_eth(dest);
    let from = dev_kp.public_key().to_ethereum_address_typed().expect("");
    let tx = eth.create_transaction_typed_inner(
        &from,
        &destination,
        amt,
        None
    ).await.expect("works");
    let gas_cost = eth.get_gas_cost_estimate(&tx).await.expect("works");
    let gas_price = eth.get_gas_price().await.expect("works");
    let fee = gas_cost.clone() * gas_price.clone();

    println!("Fee: {}", fee.json_or());
    println!("Fee: {}", fee.to_fractional());
    println!("Fee USD: {}", fee.to_fractional() * 2600.0);
    println!("Gas cost: {}", gas_cost.json_or());
    println!("Gas price: {}", gas_price.json_or());
}