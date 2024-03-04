use std::collections::{HashMap, HashSet};
use std::time::Duration;
use itertools::Itertools;
use log::info;
use redgold_keys::address_external::ToEthereumAddress;
use redgold_keys::eth::example::{dev_ci_kp, EthHistoricalClient, EthWalletWrapper};
use redgold_keys::proof_support::ProofSupport;
use redgold_keys::TestConstants;
use redgold_schema::{bytes_data, EasyJson, ProtoHashable, ProtoSerde, SafeOption, structs, WithMetadataHashable};
use redgold_schema::structs::{ControlMultipartyKeygenResponse, ControlMultipartySigningRequest, ErrorInfo, Hash, InitiateMultipartySigningRequest, NetworkEnvironment, Proof, Seed, TestContractInternalState};
use crate::api::control_api::ControlClient;
use crate::api::public_api::PublicClient;
use crate::api::RgHttpClient;
use crate::core::relay::Relay;
use crate::e2e::tx_submit::TransactionSubmitter;
use crate::multiparty::initiate_mp::default_room_id_signing;
use crate::multiparty::watcher::DepositWatcherConfig;
use crate::node::Node;
use crate::node_config::NodeConfig;
use crate::util;
use crate::observability::logging::Loggable;
use crate::observability::metrics_registry;

#[allow(dead_code)]
pub struct LocalTestNodeContext {
    id: u16,
    port_offset: u16,
    node: Node,
    public_client: PublicClient,
    control_client: ControlClient,
    // futures: Vec<JoinHandle<Result<(), ErrorInfo>>>
}

impl LocalTestNodeContext {
    async fn new(id: u16, random_port_offset: u16, seed: Vec<Seed>) -> Self {
        let mut node_config = NodeConfig::from_test_id(&id);
        node_config.port_offset = random_port_offset;
        if id == 0 {
            node_config.genesis = true;
        }

        node_config.seeds = seed.clone();
        // let runtimes = NodeRuntimes::default();
        let relay = Relay::new(node_config.clone()).await;
        Node::prelim_setup(relay.clone()
                           // , runtimes.clone()
        ).await.expect("prelim");
        // info!("Test starting node services");
        let futures = Node::start_services(relay.clone()).await;
        tokio::spawn(async move {
            let (res, _, _) = futures::future::select_all(futures).await;
            panic!("Node service failed in test: {:?}", res);
        });
        // info!("Test completed starting node services");

        let result = Node::from_config(relay.clone()).await;
        // , runtimes
        // ).await;
        let node = result
            // .await
            .expect("Node start fail");
        Self {
            id,
            port_offset: random_port_offset,
            node: node.clone(),
            public_client: PublicClient::local(node.relay.node_config.public_port(), Some(relay.clone())),
            control_client: ControlClient::local(node.relay.node_config.control_port()),
            // futures
        }
    }
}

async fn throw_panic() {
    panic!("that happened");
}
//
// #[test]
// fn test_panic() {
//     let runtime = build_runtime(1, "panic");
//     let jh = runtime.spawn(throw_panic());
//     runtime.block_on(jh).expect("Fail");
// }

struct LocalNodes {
    nodes: Vec<LocalTestNodeContext>,
    current_seed: Seed,
    seeds: Vec<Seed>,
}

impl LocalNodes {
    // fn shutdown(&self) {
    //     for x in &self.nodes {
    //         x.node.runtimes.shutdown();
    //         for jh in &x.futures {
    //             jh.abort();
    //             // std::mem::drop(jh);
    //         }
    //     }
    // }

    fn start(&self) -> &LocalTestNodeContext {
        self.nodes.get(0).unwrap()
    }
    fn current_seed_id(&self) -> u16 {
        self.nodes.len() as u16
    }
    fn seeds() -> Vec<Seed> {
        let mut seeds = vec![];
        for _i in 0..3 {
            let mut seed = NodeConfig::from_test_id(&0).self_seed();
            seed.port_offset = Some(util::random_port() as u32);
            seeds.push(seed);
        }
        seeds
    }

    async fn new(
        _offset: Option<u16>
    ) -> LocalNodes {
        // TODO: Lets avoid this and write them out to disk, or is that done already and this can be removed?
        // let path = NodeConfig::memdb_path(&(0 as u16));
        // let store =
        //     // runtime.block_on(
        //         DataStore::from_path(path).await;//);
        // let connection =
        //     // runtime.block_on(
        //         store.create_all_err_info()
        //     // )
        //     .expect("test failure create tables");
        let seeds = Self::seeds();
        let s = seeds.get(0).expect("").port_offset.expect("") as u16;
        let start = LocalTestNodeContext::new(0, s, Self::seeds()).await;
        LocalNodes {
            current_seed: start.node.relay.node_config.self_seed(),
            nodes: vec![start],
            seeds,
        }
    }

    pub fn clients(&self) -> Vec<RgHttpClient> {
        self.nodes.iter().map(|x| x.public_client.client_wrapper().clone()).collect_vec()
    }

    async fn verify_peers(&self) -> Result<(), ErrorInfo> {
        let clients = self.nodes.iter().map(|n| n.public_client.client_wrapper()).collect_vec();
        let mut map: HashMap<structs::PublicKey, Vec<structs::PeerNodeInfo>> = HashMap::new();
        for x in &clients {
            let response = x.get_peers().await?;
            let pk = response.proof.safe_get()?.public_key.safe_get()?.clone();
            let peers = response.get_peers_info_response.safe_get()?.peer_info.clone();
            map.insert(pk, peers);
        }
        let uniques = map.keys().map(|x| x.clone()).collect_vec();
        for (k, m) in map {
            for p in m {
                let nmd = p.latest_node_transaction.safe_get()?.node_metadata()?;
                let pk = nmd.public_key.safe_get()?;
                if !uniques.contains(pk) && pk != &k {
                    return Err(ErrorInfo::error_info("Peer not found in all peers"));
                }
            }
        }
        Ok(())
    }

    async fn verify_data_equivalent(&self) {
        let mut txs: Vec<HashSet<Vec<u8>>> = vec![];
        let mut obs: Vec<HashSet<Vec<u8>>> = vec![];
        let mut oes: Vec<HashSet<Vec<Vec<u8>>>> = vec![];
        let mut utxos: Vec<HashSet<Vec<u8>>> = vec![];
        let end_time = util::current_time_millis();
        for n in &self.nodes {
            let tx: HashSet<Vec<u8>> = n
                .node
                .relay
                .ds
                .transaction_store
                .query_time_transaction(0, end_time as i64).await
                .unwrap()
                .into_iter()
                .map(|x| x.transaction.unwrap().hash_vec())
                .collect();

            info!("Num tx: {:?} node_id: {:?}", tx.len(), n.id);
            txs.push(tx);

            let ob: HashSet<Vec<u8>> = n
                .node
                .relay
                .ds
                .observation
                .query_time_observation(0, end_time as i64)
                .await
                .unwrap()
                .into_iter()
                .map(|x| x.observation.unwrap().proto_serialize())
                .collect();

            info!("Num ob: {:?} node_id: {:?}", ob.len(), n.id);
            obs.push(ob);

            let oe: HashSet<Vec<Vec<u8>>> = n
                .node
                .relay
                .ds
                .observation.query_time_observation_edge(0, end_time as i64)
                .await
                .unwrap()
                .into_iter()
                .map(|x| {
                    let proof = x.observation_proof.unwrap();
                    let proof1 = proof.merkle_proof.unwrap();
                    let x1 = proof.metadata.unwrap().clone();
                    vec![proof1.root.unwrap().clone().vec(), x1.observed_hash.unwrap().clone().vec(), proof.observation_hash.unwrap().clone().vec()]
                })
                .collect();
            info!("Num oe: {:?} node_id: {:?}", oe.len(), n.id);
            oes.push(oe);

            let utxo: HashSet<Vec<u8>> = n
                .node
                .relay
                .ds
                .transaction_store
                .utxo_filter_time(0, end_time as i64)
                .await
                .unwrap()
                .into_iter()
                .map(|x| {
                    x.output
                        .unwrap()
                        .calculate_hash()
                        .bytes
                        .expect("b")
                        .value
                })
                .collect();
            info!("Num utxos: {:?} node_id: {:?}", utxo.len(), n.id);

            utxos.push(utxo);
        }
        for x in txs.clone() {
            assert_eq!(x, txs.get(0).unwrap().clone());
        }
        for x in obs.clone() {
            assert_eq!(x, obs.get(0).unwrap().clone());
        }
        for x in oes.clone() {
            assert_eq!(x, oes.get(0).unwrap().clone());
        }
        for x in utxos.clone() {
            assert_eq!(x, utxos.get(0).unwrap().clone());
        }
    }

    async fn add_node(&mut self) {
        let port_offset = util::random_port();
        // let path = NodeConfig::memdb_path(&self.current_seed_id());
        // let store = DataStore::from_path(path).await;
        // let connection =
        //     // runtime.block_on(
        //     store.create_all_err_info()
        //         // )
        //         .expect("test failure create tables");
        let start = LocalTestNodeContext::new(
            self.current_seed_id(),
            port_offset,
            self.seeds.clone(),
        ).await;

        info!(
            "Number of transactions after localnodetestcontext {}",
            start
                .node
                .relay
                .ds
                .transaction_store
                .query_time_transaction(0, util::current_time_millis_i64()).await
                .unwrap()
                .len()
        );

        self.nodes.push(start);
        // self.connections.push(connection);
    }
}
//
// #[ignore]
// #[test]
// fn debug_err() {
//
//     let runtime = build_runtime(10, "e2e");
//     util::init_logger().ok(); //.expect("log");
//     metrics_registry::register_metric_names();
//     metrics_registry::init_print_logger();
//     let _tc = TestConstants::new();
//
//     // testing part here debug
//     let mut node_config = NodeConfig::from_test_id(&0);
//     node_config.port_offset = 15000;
//     let runtimes = NodeRuntimes::default();
//     let mut relay = runtimes.auxiliary.block_on(Relay::new(node_config.clone()));
//     Node::prelim_setup(relay.clone()
//                        // , runtimes.clone()
//     ).expect("prelim");
//     // Node::start_services(relay.clone(), runtimes.clone());
//     let result = Node::from_config(relay.clone(), runtimes);
//     info!("wtf");
//     let node = result;
//         // .expect("Node start fail");
//
//     match node {
//         Ok(_) => {
//             info!("Success");
//         }
//         Err(e) => {
//             info!("Node result: {:?}", e);
//         }
//     }
//
//
// }


/// Main entry point for end to end testing.
// #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[tokio::test]
async fn e2e() {
    e2e_async(false).await.expect("");
    // let runtime = build_runtime(8, "e2e");
    // runtime.block_on(e2e_async()).expect("e2e");
}


async fn e2e_async(contract_tests: bool) -> Result<(), ErrorInfo> {
    util::init_logger_once();
    metrics_registry::register_metric_names();
    // metrics_registry::init_print_logger();
    // init_tracing();
    let _tc = TestConstants::new();

    let mut local_nodes = LocalNodes::new(None).await;
    let start_node = local_nodes.start();
    // info!("Started initial node");
    let client1 = start_node.control_client.clone();
    let _client2 = start_node.control_client.clone();
    let _ds = start_node.node.relay.ds.clone();
    // let show_balances = || {
    //     let res = &ds.query_all_balance();
    //     let res2 = res.as_ref().unwrap();
    //     let str = serde_json::to_string(&res2).unwrap();
    //     info!("Balances: {}", str);
    // };

    // show_balances();

    let client = start_node.public_client.clone();

    for u in start_node.node.relay.ds.transaction_store.utxo_all_debug().await.expect("utxo all debug") {
        info!("utxo at start: {}", u.json_or());
    }
    //
    // let utxos = ds.query_time_utxo(0, util::current_time_millis())
    //     .unwrap();
    // info!("Num utxos from genesis {:?}", utxos.len());

    let (_, spend_utxos) = Node::genesis_from(start_node.node.relay.node_config.clone());

    let submit = TransactionSubmitter::default(client.clone(),
                                               // runtime.clone(),
                                               spend_utxos,
        &NetworkEnvironment::Debug
    );

    submit.submit().await.expect("submit");

    if contract_tests {
        let res = submit.submit_test_contract().await.expect("submit test contract");
        let ct = res.transaction.expect("tx");
        let contract_address = ct.first_output_address().expect("cont");
        let _o = ct.outputs.get(0).expect("O");
        let state = client.client_wrapper().contract_state(&contract_address).await.expect("res");
        let state_json = TestContractInternalState::proto_deserialize(state.state.clone().expect("").value).expect("").json_or();
        info!("First contract state marker: {} {}", state.json_or(), state_json);

        submit.submit_test_contract_call(&contract_address ).await.expect("worx");
        let state = client.client_wrapper().contract_state(&contract_address).await.expect("res");
        let state_json = TestContractInternalState::proto_deserialize(state.state.clone().expect("").value).expect("").json_or();
        info!("Second contract state marker: {} {}", state.json_or(), state_json);
        return Ok(());
    }


    // let utxos = ds.query_time_utxo(0, util::current_time_millis())
    //     .unwrap();
    // info!("Num utxos after first submit {:?}", utxos.len());


    // Exception bad access on this on the json decoding? wtf?
    let _ = submit.with_faucet().await.expect("faucet");
    // info!("Faucet response: {}", faucet_res.json_pretty_or());

    submit.submit().await.expect("submit 2");

    // info!("Num utxos after second submit {:?}", utxos.len());

    submit.submit_duplicate().await;

    // info!("Num utxos after duplicate submit {:?}", utxos.len());

    // show_balances();
    // // shouldn't response metadata be not an option??

    for _ in 0..1 {
        // TODO Flaky failure observed once? Why?
        submit.submit_double_spend(None).await;
    }

    submit.submit_invalid_signature().await;

    // info!("Num utxos after double spend submit {:?}", utxos.len());

    // show_balances();

    // for _ in 0..2 {
    //     submit.submit_split();
    //     show_balances();
    // }

    // let addr =
    //     runtime.block_on(
    // client.query_addresses(submit.get_addresses()).await;

    // info!("Address response: {:?}", addr);

    // //
    // let after_node_added = submit.submit();
    // assert_eq!(2, after_node_added.submit_transaction_response.expect("submit").query_transaction_response.expect("query")
    //     .observation_proofs.len());


    local_nodes.verify_data_equivalent().await;

    local_nodes.add_node(
        // runtime.clone()
    ).await;
    local_nodes.verify_data_equivalent().await;

    tokio::time::sleep(Duration::from_secs(2)).await;

    let after_2_nodes = submit.submit().await.expect("submit");

    // tracing::info!("After two nodes started first submit: {}", after_2_nodes.json_pretty_or());

    // Debug for purpose of viewing 2nd node operations
    // tokio::time::sleep(Duration::from_secs(15)).await;
    after_2_nodes.at_least_n(2).unwrap();

    local_nodes.verify_peers().await.expect("verify peers");


    let keygen1 = client1.multiparty_keygen(None).await.log_error()?;

    // tokio::time::sleep(Duration::from_secs(10)).await;


    let signing_data = Hash::from_string_calculate("hey");
    let _result = do_signing(keygen1.clone(), signing_data.clone(), client1.clone()).await;

    tracing::info!("After MP test");

    submit.with_faucet().await.unwrap().submit_transaction_response.expect("").at_least_n(2).unwrap();

    local_nodes.verify_data_equivalent().await;

    // three nodes
    local_nodes.add_node().await;
    local_nodes.verify_data_equivalent().await;
    local_nodes.verify_peers().await?;

    // This works but is really flaky for some reason?
    // submit.with_faucet().await.unwrap().submit_transaction_response.expect("").at_least_n(3).unwrap();

    // submit.submit().await?.at_least_n(3).unwrap();

    let keygen2 = client1.multiparty_keygen(None).await.log_error()?;
    let res = do_signing(keygen2.clone(), signing_data.clone(), client1.clone()).await;
    let public = res.public_key.expect("public key");
    let mp_eth_addr = public.to_ethereum_address().expect("eth address");

    let environment = NetworkEnvironment::Dev;


    let do_mp_eth_test = false;

    if do_mp_eth_test {
        // Ignore this part for now
        let h = EthHistoricalClient::new(&environment).expect("works").expect("works");
        let string_addr = "0xA729F9430fc31Cda6173A0e81B55bBC92426f759".to_string();
        let txs = h.get_all_tx(&string_addr).await.expect("works");
        println!("txs: {}", txs.json_or());
        let tx_head = txs.get(0).expect("tx");
        let _other_address = tx_head.other_address.clone();

        // Load using the faucet KP, but send to the multiparty address
        let (dev_secret, dev_kp) = dev_ci_kp().expect("works");
        let eth = EthWalletWrapper::new(&dev_secret, &environment).expect("works");
        let dev_faucet_rx_addr = dev_kp.public_key().to_ethereum_address().expect("works");
        let fee = "0.000108594791676".to_string();
        let fee_value = EthHistoricalClient::translate_float_value(&fee.to_string()).expect("works") as u64;
        let amount = fee_value * 6;
        let _tx = eth.send_tx(&mp_eth_addr, amount).await.expect("works");

        tokio::time::sleep(Duration::from_secs(20)).await;

        let mut tx = eth.create_transaction(&mp_eth_addr, &dev_faucet_rx_addr, fee_value * 3).await.expect("works");
        let data = EthWalletWrapper::signing_data(&tx).expect("works");
        let h = Hash::new(data);
        let res = do_signing(keygen2.clone(), h.clone(), client1.clone()).await;
        let sig = res.signature.expect("sig");
        let raw = EthWalletWrapper::process_signature(sig, &mut tx).expect("works");
        eth.broadcast_tx(raw).await.expect("works");
    }
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



    std::mem::forget(local_nodes);
    std::mem::forget(submit);
    Ok(())
}

#[ignore]
#[test]
fn env_var_parse_test() {

    println!("Env var test")

}

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
        relay.ds.transaction_store.insert_transaction(&tx, 0,true, None, true).await.expect("");
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

async fn do_signing(party: ControlMultipartyKeygenResponse, signing_data: Hash, client: ControlClient) -> Proof {

        let vec1 = signing_data.vec();
        let vec = bytes_data(vec1.clone()).expect("");
        let mut signing_request = ControlMultipartySigningRequest::default();
        let mut init_signing = InitiateMultipartySigningRequest::default();
        let identifier = party.multiparty_identifier.expect("");
        init_signing.signing_room_id = default_room_id_signing(identifier.uuid.clone());
        init_signing.data_to_sign = Some(vec);
        init_signing.identifier = Some(identifier.clone());
        signing_request.signing_request = Some(init_signing);
        let res =
            client.multiparty_signing(signing_request).await;
        // println!("{:?}", res);
        assert!(res.is_ok());
        let proof = res.expect("ok").proof.expect("prof");
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
