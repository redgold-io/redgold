use std::collections::HashSet;
use std::path::Path;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use futures::stream::FuturesUnordered;
use itertools::Itertools;

use log::info;
use metrics::increment_counter;
use rusqlite::Connection;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use redgold_schema::constants::REWARD_AMOUNT;
use redgold_schema::{bytes_data, EasyJson, error_info, ProtoSerde, SafeBytesAccess, SafeOption};
use redgold_schema::structs::{GetPeersInfoRequest, Hash, NetworkEnvironment, Request, Transaction};

use crate::api::control_api::ControlClient;
use crate::api::p2p_io::rgnetwork::Event;
use crate::api::p2p_io::P2P;
use crate::api::public_api;
use crate::api::public_api::PublicClient;
use crate::api::{control_api, rosetta};
use crate::canary::tx_submit::TransactionSubmitter;
use crate::core::block_formation;
use crate::core::block_formation::BlockFormationProcess;
use crate::core::observation::ObservationBuffer;
use crate::core::peer_event_handler::PeerOutgoingEventHandler;
use crate::core::peer_rx_event_handler::{PeerRxEventHandler, rest_peer};
use crate::core::process_transaction::TransactionProcessContext;
use crate::core::relay::Relay;
use crate::data::data_store::DataStore;
use crate::data::download;
use crate::genesis::{create_genesis_transaction, genesis_tx_from, GenesisDistribution};
use crate::node_config::NodeConfig;
use crate::schema::structs::{ ControlRequest, ErrorInfo, NodeState};
use crate::schema::{ProtoHashable, WithMetadataHashable};
use crate::trust::rewards::Rewards;
use crate::{canary, util};
use crate::mparty::mp_server::{Db, MultipartyHandler};
use crate::canary::tx_gen::SpendableUTXO;
use crate::core::process_observation::ObservationHandler;
use crate::core::seeds::SeedNode;
use crate::multiparty::gg20_sm_manager;
use crate::util::runtimes::build_runtime;
use crate::util::{auto_update, keys, metrics_registry};
use crate::schema::constants::EARLIEST_TIME;
use crate::schema::TestConstants;
use crate::util::trace_setup::init_tracing;

#[derive(Clone)]
pub struct Node {
    pub relay: Relay,
    pub runtimes: NodeRuntimes,
}

#[derive(Clone)]
pub struct NodeRuntimes {
    p2p: Arc<Runtime>,
    pub(crate) public_api: Arc<Runtime>,
    control_api: Arc<Runtime>,
    transaction_process_context: Arc<Runtime>,
    transaction_process: Arc<Runtime>,
    pub(crate) auxiliary: Arc<Runtime>,
    pub canary_watcher: Arc<Runtime>,
    pub async_multi: Arc<Runtime>
}

impl NodeRuntimes {
    pub fn shutdown(&self) {
        // self.p2p.deref().shutdown_background();
        // self.public_api.shutdown_background();
        // self.control_api.shutdown_background();
        // self.transaction_process_context.shutdown_background();
        // self.transaction_process.shutdown_background();
        // self.auxiliary.shutdown_background();
    }
    pub fn default() -> Self {
        Self {
            p2p: build_runtime(4, "p2p"),
            public_api: build_runtime(4, "public"),
            control_api: build_runtime(4, "control"),
            transaction_process_context: build_runtime(4, "transaction_process_context"),
            transaction_process: build_runtime(4, "transaction_process"),
            auxiliary: build_runtime(8, "aux"),
            canary_watcher: build_runtime(1, "canary_watcher"),
            async_multi: build_runtime(5, "async_multi"),
        }
    }
}

struct NodeInit {
    relay: Relay
}

impl NodeInit {

}

impl Node {

    pub fn start_services(relay: Relay, runtimes: NodeRuntimes) -> Vec<JoinHandle<Result<(), ErrorInfo>>> {
        let mut join_handles = vec![];
        let node_config = relay.node_config.clone();

        // let (p2p_rx_s, p2p_rx_r) = futures::channel::mpsc::channel::<Event>(1000);

        // Concurrent processes
        // let (p2p, jh_p2p) = P2P::new(relay.clone(), runtimes.p2p.clone(), p2p_rx_s);
        // join_handles.extend(jh_p2p);

        let jh_ctrl = control_api::ControlServer {
            relay: relay.clone(),
            // p2p_client: p2p.client.clone(),
            runtime: runtimes.control_api.clone(),
        }
            .start();

        join_handles.push(jh_ctrl);

        let peer_tx_jh = PeerOutgoingEventHandler::new(relay.clone(), runtimes.p2p.clone());
        join_handles.push(peer_tx_jh);

        let tx_p_jh = TransactionProcessContext::new(
            relay.clone(),
            runtimes.transaction_process_context.clone(),
            runtimes.transaction_process.clone(),
        );
        join_handles.push(tx_p_jh);

        // TODO: Replace with readiness probe.
        runtimes
            .auxiliary
            .block_on(async { sleep(Duration::new(3, 1)).await });


        // TODO: Monitor this join handle for errors.
        // runtimes
        //     .auxiliary
        //     .spawn(auto_update::from_node_config(node_config.clone()));
        //

        // Components for download now initialized.
        // relay.clone().node_state.store(NodeState::Downloading);


        let ojh = ObservationBuffer::new(relay.clone(), runtimes.auxiliary.clone());
        join_handles.push(ojh);

        // Rewards::new(relay.clone(), runtimes.auxiliary.clone());

        join_handles.push(PeerRxEventHandler::new(
            relay.clone(),
            runtimes.auxiliary.clone(),
        ));

        join_handles.push(public_api::start_server(relay.clone(), runtimes.public_api.clone()));
        let obs_handler = ObservationHandler{relay: relay.clone()};
        join_handles.push(runtimes.auxiliary.spawn(async move { obs_handler.run().await }));

        let mut mph = MultipartyHandler::new(
            relay.clone(),
            runtimes.auxiliary.clone()
        );
        join_handles.push(runtimes.auxiliary.spawn(async move { mph.run().await }));

        let sm_port = relay.node_config.mparty_port();
        join_handles.push(runtimes.auxiliary
            .spawn(async move { gg20_sm_manager::run_server(sm_port)
                .await.map_err(|e| error_info(e.to_string())) }));


        let relay_c = relay.clone();
        // let amh = runtimes.async_multi.spawn(async move {
        //     let r = relay_c.clone();
        //     let blocks = BlockFormationProcess::default(r.clone()).await?;
        //     // TODO: Select from list of futures.
        //     Ok::<(), ErrorInfo>(tokio::select! {
        //         res = blocks.run() => {res?}
        //         // res = obs_handler.run() => {res?}
        //         // _ = rosetta::run_server(r.clone()) => {}
        //         // _ = public_api::run_server(r.clone()) => {}
        //     })
        // });
        // join_handles.push(amh);
        let c_config = relay.clone();
        if node_config.canary_enabled {
            let cwh = runtimes.canary_watcher.spawn_blocking(move || { canary::run(c_config) });
            join_handles.push(cwh);
        }


        join_handles
    }

    pub fn prelim_setup(relay2: Relay, runtimes: NodeRuntimes) -> Result<(), ErrorInfo> {
        let mut relay = relay2.clone();
        let node_config = relay.node_config.clone();

        let migration_result = runtimes.auxiliary.block_on(relay.ds.run_migrations());
        if let Err(e) = migration_result {
            log::error!("Migration related failure, attempting to handle");
            if e.message.contains("was previously applied") &&
                node_config.clone().network != NetworkEnvironment::Main {
                log::error!("Found prior conflicting schema -- but test environment, removing existing datastore and exiting");

                std::fs::remove_file(Path::new(&node_config.data_store_path))
                    .map_err(|e| error_info(format!("Couldn't remove existing datastore: {}", e.to_string())))?;
                panic!("Exiting due to ds removal, should work on retry");
                // relay = runtimes.auxiliary.block_on(Relay::new(node_config.clone()));
                // runtimes.auxiliary.block_on(relay.ds.run_migrations())?;
            } else {
                return Err(e);
            }
        }
        Ok(())
    }

    pub fn throw_error() -> Result<(), ErrorInfo> {
        Err(ErrorInfo::error_info("test"))?;
        Ok(())
    }

    pub fn throw_error_panic() -> Result<(), ErrorInfo> {
        let result3: Result<Node, ErrorInfo> = Err(ErrorInfo::error_info("test"));
        result3.expect("expected panic");
        Ok(())
    }
    //
    // pub async fn initial_peers_request(&self, seed: SeedNode) -> Result<(), ErrorInfo> {
    //
    //     let all_peers = self.relay.ds.peer_store.all_peers().await?;
    //     let set = all_peers.iter().map(|p| p.peer_id.as_ref().expect("p").clone()).collect::<HashSet<Vec<u8>>>();
    //     let mut req = self.relay.node_config.request();
    //     req.get_peers_info_request = Some(GetPeersInfoRequest{});
    //     let res = rest_peer(
    //         self.relay.node_config.clone(), seed.external_address.clone(), (seed.port + 1) as i64, req
    //     ).await?; // TODO: Use retries here, and don't consider it a failure if we can't get peers immediately
    //     //self.relay.ds.peer_store.
    //     let option = res.get_peers_info_response.clone();
    //     let peers = option.expect("peers").peers;
    //     // TODO: Validate peers added.
    //     for peer_tx in peers {
    //         let pd = peer_tx.peer_data()?;
    //         let pid = pd.peer_id.safe_get()?;
    //         if !set.contains(pid) {
    //             self.relay.ds.peer_store.add_peer(&peer_tx, 0f64).await?;
    //         }
    //     }
    //     Ok(())
    //
    // }

    pub fn genesis_from(node_config: NodeConfig) -> (Transaction, Vec<SpendableUTXO>) {
        let outputs = (0..50).map(|i|
            GenesisDistribution {
                address: node_config.wallet().key_at(i as usize).address_typed(), amount: 10000
            }
        ).collect_vec();
        let tx = genesis_tx_from(outputs); //EARLIEST_TIME
        let res = tx.to_utxo_entries(EARLIEST_TIME as u64).iter().zip(0..50).map(|(o, i)| {
            let kp = node_config.wallet().key_at(i as usize);
            let s = SpendableUTXO {
                utxo_entry: o.clone(),
                key_pair: kp,
            };
            s
        }).collect_vec();
        (tx, res)
    }

    pub fn from_config(relay: Relay, runtimes: NodeRuntimes) -> Result<Node, ErrorInfo> {
        // Inter thread communication

        let mut node = Self {
            relay: relay.clone(),
            runtimes: runtimes.clone(),
        };

        let node_config = relay.node_config.clone();

        log::debug!("Select all DS tables: {:?}", relay.ds.select_all_tables());

        let result1 = std::env::var("REDGOLD_GENESIS");
        log::debug!("REDGOLD_GENESIS environment variable: {:?}", result1);

        let flag_genesis = result1
            .map(|g| g.replace("\"", "").trim().to_string() == "true".to_string())
            .unwrap_or(false) && node_config.main_stage_network();
        let debug_genesis = !node_config.main_stage_network() && relay.node_config.seeds.is_empty();

        if flag_genesis || debug_genesis {
            info!("No seeds configured, starting from genesis");
            // relay.node_state.store(NodeState::Ready);
            // TODO: Replace with genesis per network type.
            // if node_config.is_debug() {
            //     info!("Genesis code kp");
            //     let _res_err = DataStore::map_err(
            //         relay
            //             .ds
            //             .insert_transaction(&create_genesis_transaction(), EARLIEST_TIME),
            //     );
            // } else {
                info!("Genesis local test multiple kp");

                let tx = Node::genesis_from(node_config.clone()).0;
                runtimes.auxiliary.block_on(relay.ds.config_store.insert_update_json("genesis", tx.json()?))?;
                let _res_err = runtimes.auxiliary.block_on(
                    relay
                        .ds
                        .transaction_store
                        .insert_transaction(&tx, EARLIEST_TIME, true, None)
                );
            // }
            // .expect("Genesis inserted or already exists");

        } else {

            info!("Starting from seed nodes");
            let seed = if node_config.main_stage_network() {
                info!("Querying LB for node info");
                let a = runtimes.auxiliary.block_on(node_config.lb_client().about())?;
                let tx = a.latest_metadata.safe_get_msg("Missing latest metadata from seed node")?;
                let pd = tx.outputs.get(0).expect("a").data.as_ref().expect("d").peer_data.as_ref().expect("pd");
                let nmd = pd.node_metadata.get(0).expect("nmd");
                let vec = nmd.public_key_bytes().expect("ok");
                let vec1 = pd.peer_id.safe_get()?.clone().peer_id.safe_bytes()?.clone();
                SeedNode{
                    peer_id: Some(vec1),
                    trust: vec![],
                    public_key: Some(keys::public_key_from_bytes(&vec).expect("pk")),
                    external_address: nmd.external_address.clone(),
                    port_offset: Some(nmd.port_offset.unwrap_or(node_config.network.default_port_offset() as i64) as u16),
                    environments: vec![],
                }
            } else {
                relay.node_config.seeds.get(0).unwrap().clone()

            };
            let port = seed.port_offset.unwrap() + 1;
            let client = PublicClient::from(seed.external_address.clone(), port);
            info!("Querying with public client for node info again on: {} : {:?}", seed.external_address, port);
            let result = runtimes.auxiliary.block_on(client.about());
            let peer_tx = result?.latest_metadata.safe_get()?.clone();

            info!("Got LB node info {}, adding peer", redgold_schema::json(&peer_tx)?);
            // Local debug mode
            // First attempt to insert all trust scores for seeds and ignore conflict
            let result2 = runtimes.auxiliary.block_on(relay.ds.peer_store.add_peer(&peer_tx, 1f64));
            info!("Peer add result: {:?}", result2);
            result2?;



            info!("Added peer, attempting download");
            // relay
            //     .ds
            //     .insert_peer_single(
            //         &seed.peer_id,
            //         seed.trust,
            //         &seed.public_key.serialize().to_vec().clone(),
            //         seed.external_address.clone(),
            //     )
            //     .expect("insert peer on download");
            // todo: send_peer_request_response
            let data = peer_tx.peer_data()?;
            let key = data.node_metadata[0].public_key_bytes()?;
            // TODO Change this invocation to an .into() in a non-schema key module
            let pk = keys::public_key_from_bytes(&key).expect("works");
            runtimes.auxiliary.block_on(download::download(
                relay.clone(),
                pk
            ));
        }

        info!("Node ready");
        increment_counter!("redgold.node.node_started");

        return Ok(node);
    }
}

#[allow(dead_code)]
pub struct LocalTestNodeContext {
    id: u16,
    port_offset: u16,
    node: Node,
    public_client: PublicClient,
    control_client: ControlClient,
    futures: Vec<JoinHandle<Result<(), ErrorInfo>>>
}

impl LocalTestNodeContext {
    fn new(id: u16, random_port_offset: u16, seed: Option<SeedNode>) -> Self {
        let mut node_config = NodeConfig::from_test_id(&id);
        node_config.port_offset = random_port_offset;
        for x in seed {
            node_config.seeds = vec![x];
        }
        let runtimes = NodeRuntimes::default();
        let mut relay = runtimes.auxiliary.block_on(Relay::new(node_config.clone()));
        Node::prelim_setup(relay.clone(), runtimes.clone()).expect("prelim");
        let futures = Node::start_services(relay.clone(), runtimes.clone());
        let result = Node::from_config(relay.clone(), runtimes);
        let node = result
            // .await
            .expect("Node start fail");
        Self {
            id,
            port_offset: random_port_offset,
            node: node.clone(),
            public_client: PublicClient::local(node.relay.node_config.public_port()),
            control_client: ControlClient::local(node.relay.node_config.control_port()),
            futures
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
    connections: Vec<Connection>,
    current_seed: SeedNode,
}

impl LocalNodes {
    fn shutdown(&self) {
        for x in &self.nodes {
            x.node.runtimes.shutdown();
            for jh in &x.futures {
                jh.abort();
                // std::mem::drop(jh);
            }
        }
    }

    fn start(&self) -> &LocalTestNodeContext {
        self.nodes.get(0).unwrap()
    }
    fn current_seed_id(&self) -> u16 {
        self.nodes.len() as u16
    }
    fn new(runtime: Arc<Runtime>, offset: Option<u16>) -> LocalNodes {
        let port_offset = offset.unwrap_or(util::random_port());
        let path = NodeConfig::memdb_path(&(0 as u16));
        let store = runtime.block_on(DataStore::from_path(path));
        // let connection =
        //     // runtime.block_on(
        //         store.create_all_err_info()
        //     // )
        //     .expect("test failure create tables");
        let start = LocalTestNodeContext::new(0, port_offset, None); //.await;
        LocalNodes {
            connections: vec![], //connection],
            current_seed: SeedNode {
                peer_id: Some(start.node.relay.node_config.clone().self_peer_id),
                trust: vec![],
                public_key: Some(start
                    .node
                    .relay
                    .node_config
                    .clone()
                    .wallet()
                    .transport_key()
                    .public_key
                ),
                external_address: start.node.relay.node_config.external_ip.clone(),
                port_offset: Some(start.node.relay.node_config.port_offset),
                environments: vec![],
            },
            nodes: vec![start],
        }
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
                .query_time_transaction(0, end_time)
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
                .query_time_observation(0, end_time)
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
                .query_time_utxo(0, end_time)
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

    fn add_node(&mut self, runtime: Arc<Runtime>) {
        let port_offset = util::random_port();
        let path = NodeConfig::memdb_path(&self.current_seed_id());
        let store = runtime.block_on(DataStore::from_path(path));
        let connection =
            // runtime.block_on(
            store.create_all_err_info()
                // )
                .expect("test failure create tables");
        let start = LocalTestNodeContext::new(
            self.current_seed_id(),
            port_offset,
            Some(self.current_seed.clone()),
        );
        // .await;

        info!(
            "Number of transactions after localnodetestcontext {}",
            start
                .node
                .relay
                .ds
                .query_time_transaction(0, util::current_time_millis())
                .unwrap()
                .len()
        );

        self.nodes.push(start);
        self.connections.push(connection);
    }
}

#[ignore]
#[test]
fn debug_err() {

    let runtime = build_runtime(10, "e2e");
    util::init_logger().ok(); //.expect("log");
    metrics_registry::register_metric_names();
    metrics_registry::init_print_logger();
    let _tc = TestConstants::new();

    // testing part here debug
    let mut node_config = NodeConfig::from_test_id(&0);
    node_config.port_offset = 15000;
    let runtimes = NodeRuntimes::default();
    let mut relay = runtimes.auxiliary.block_on(Relay::new(node_config.clone()));
    Node::prelim_setup(relay.clone(), runtimes.clone()).expect("prelim");
    // Node::start_services(relay.clone(), runtimes.clone());
    let result = Node::from_config(relay.clone(), runtimes);
    info!("wtf");
    let node = result;
        // .expect("Node start fail");

    match node {
        Ok(_) => {
            info!("Success");
        }
        Err(e) => {
            info!("Node result: {:?}", e);
        }
    }


}

#[test]
fn e2e() {
    // hot dog
    // let do_run = std::env::var("CI");
    // if do_run.is_ok() {
    //     return;
    // }

    let runtime = build_runtime(10, "e2e");
    util::init_logger().ok(); //.expect("log");
    metrics_registry::register_metric_names();
    metrics_registry::init_print_logger();
    // init_tracing();
    let _tc = TestConstants::new();


    // runtime.block_on(async { sleep(Duration::new(3, 1)).await });

    //
    let mut local_nodes = LocalNodes::new(runtime.clone(), None);
    let start_node = local_nodes.start().clone();
    let client1 = start_node.control_client.clone();

    let ds = start_node.node.relay.ds.clone();
    //
    // // Await nodes started;
    //
    let show_balances = || {
        let res = &ds.query_all_balance();
        // if res.is_err() {
        //     error!("wtf: {:?}", res);
        // }
        let res2 = res.as_ref().unwrap();
        let str = serde_json::to_string(&res2).unwrap();
        info!("Balances: {}", str);
    };

    show_balances();

    let client = start_node.public_client.clone();
    //
    let utxos = ds.query_time_utxo(0, util::current_time_millis())
        .unwrap();
    info!("Num utxos from genesis {:?}", utxos.len());

    let (_, spend_utxos) = Node::genesis_from(start_node.node.relay.node_config.clone());
    let submit = TransactionSubmitter::default(client.clone(), runtime.clone(), spend_utxos);

    let _result = submit.submit();
    info!("First submit response: {}", _result.json_pretty_or());
    assert!(_result.is_ok());

    let x = _result.expect("").query_transaction_response;
    assert!(&x.is_some());
    let query = x.expect("qry");
    assert_eq!(query.observation_proofs.len(), 2);
    // assert!(response)



    let utxos = ds.query_time_utxo(0, util::current_time_millis())
        .unwrap();
    info!("Num utxos after first submit {:?}", utxos.len());


    let faucet_res = submit.with_faucet();
    info!("Faucet response: {}", faucet_res.json_pretty_or());
    assert!(faucet_res.acceptance_proofs.len() > 0);

    let _result2 = submit.submit();
    assert!(_result2.is_ok());
    show_balances();

    info!("Num utxos after second submit {:?}", utxos.len());

    submit.submit_duplicate();

    info!("Num utxos after duplicate submit {:?}", utxos.len());

    // show_balances();
    // // shouldn't response metadata be not an option??

    for _ in 0..1 {
        // TODO Flaky failure observed once? Why?
        submit.submit_double_spend(None);
    }

    info!("Num utxos after double spend submit {:?}", utxos.len());

    show_balances();

    // for _ in 0..2 {
    //     submit.submit_split();
    //     show_balances();
    // }

    let addr = runtime.block_on(client.query_addresses(submit.get_addresses()));

    info!("Address response: {:?}", addr);

    runtime.block_on(local_nodes.verify_data_equivalent());

    local_nodes.add_node(runtime.clone());
    runtime.block_on(local_nodes.verify_data_equivalent());
    // //
    // let after_node_added = submit.submit();
    // assert_eq!(2, after_node_added.submit_transaction_response.expect("submit").query_transaction_response.expect("query")
    //     .observation_proofs.len());


    let res = runtime.block_on(client1.multiparty_keygen(None));
    println!("{:?}", res);
    /*
    "protocol execution terminated with error: handle received message: received message didn't pass pre-validation: got message which was sent by this party"
    this happens sometimes?
    Do we need some kind of sleep in here before the other peers start? very confusing.
     */
    assert!(res.is_ok());

    let party = res.expect("ok");
    let signing_data = Hash::from_string("hey");
    let vec1 = signing_data.ecdsa_short_signing_bytes();
    let vec = bytes_data(vec1.clone()).expect("");
    let res = runtime.block_on(
        client1.multiparty_signing(None, party.initial_request, vec));
    println!("{:?}", res);
    assert!(res.is_ok());
    res.expect("ok").proof.expect("prof").verify(&vec1).expect("verified");


    // // Connect first peer.
    // let add_request = local_nodes.nodes.get(1).unwrap().add_request(false);
    // let add_response = runtime.block_on(start_node.control_client.request(&add_request));
    // info!("Add peer response: {:?}", add_response);
    // let add_request2 = start_node.add_request(true);
    // let add_response2 =
    //     runtime.block_on(nodes.get(1).unwrap().control_client.request(&add_request2));
    // info!("Add peer response2: {:?}", add_response2);
    //
    // // // Connect nodes together
    // // for n_i in 0..node_count {
    // //     for n_j in 0..node_count {
    // //         // info!("ni {} nj {}", n_i, n_j);
    // //         if n_i != n_j {
    // //             let n = nodes.get(n_i).unwrap();
    // //             let n2 = nodes.get(n_j).unwrap();
    // //             let request = n2.add_request();
    // //             // info!(
    // //             //     "Add peer request on {} to {} req {}",
    // //             //     n.id,
    // //             //     n2.id,
    // //             //     serde_json::to_string(&request).unwrap()
    // //             // );
    // //             let response = n.control_client.request(&request).await.unwrap();
    // //             // info!("Add peer response: {:?}", response);
    // //         }
    // //     }
    // // }
    // // info!("What the heck");
    // //

    // runtime.shutdown_background();


    local_nodes.shutdown();

    // std::mem::forget(nodes);
    std::mem::forget(local_nodes);
    std::mem::forget(runtime);
    std::mem::forget(submit);
}

#[ignore]
#[test]
fn env_var_parse_test() {

    println!("Env var test")

}