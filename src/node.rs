use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use futures::stream::FuturesUnordered;
use itertools::Itertools;

use log::info;
use metrics::increment_counter;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use redgold_schema::constants::REWARD_AMOUNT;
use redgold_schema::{bytes_data, EasyJson, error_info, ProtoSerde, SafeBytesAccess, SafeOption, structs};
use redgold_schema::structs::{ControlMultipartyKeygenResponse, ControlMultipartySigningRequest, GetPeersInfoRequest, Hash, InitiateMultipartySigningRequest, NetworkEnvironment, PeerId, Request, Seed, Transaction, TrustData};

use crate::api::control_api::ControlClient;
// use crate::api::p2p_io::rgnetwork::Event;
// use crate::api::p2p_io::P2P;
use crate::api::{RgHttpClient, public_api, explorer};
use crate::api::public_api::PublicClient;
use crate::api::{control_api, rosetta};
use crate::e2e::tx_submit::TransactionSubmitter;
use crate::core::{block_formation, stream_handlers};
use crate::core::block_formation::BlockFormationProcess;
use crate::core::observation::ObservationBuffer;
use crate::core::peer_event_handler::PeerOutgoingEventHandler;
use crate::core::peer_rx_event_handler::{PeerRxEventHandler, rest_peer};
use crate::core::process_transaction::TransactionProcessContext;
use crate::core::relay::Relay;
use redgold_data::data_store::DataStore;
use crate::data::download;
use crate::genesis::{create_genesis_transaction, genesis_tx_from, GenesisDistribution};
use crate::node_config::NodeConfig;
use crate::schema::structs::{ ControlRequest, ErrorInfo, NodeState};
use crate::schema::{ProtoHashable, WithMetadataHashable};
// use crate::trust::rewards::Rewards;
use crate::{api, e2e, util};
// use crate::mparty::mp_server::{Db, MultipartyHandler};
use crate::e2e::tx_gen::SpendableUTXO;
use crate::core::process_observation::ObservationHandler;
use crate::core::seeds::SeedNode;
use crate::multiparty::gg20_sm_manager;
use crate::util::runtimes::build_runtime;
use crate::util::{auto_update, keys, metrics_registry};
use crate::schema::constants::EARLIEST_TIME;
use redgold_keys::TestConstants;
use crate::util::trace_setup::init_tracing;
use tokio::task::spawn_blocking;
use tracing::Span;
use redgold_keys::proof_support::ProofSupport;
use crate::core::discovery::{Discovery, DiscoveryMessage};
use crate::core::internal_message::SendErrorInfo;
use crate::core::stream_handlers::IntervalFold;
use crate::multiparty::initiate_mp::default_room_id_signing;
use crate::multiparty::watcher::Watcher;
use crate::observability::dynamic_prometheus::update_prometheus_configs;
use crate::shuffle::shuffle_interval::Shuffle;
use crate::util::logging::Loggable;

#[derive(Clone)]
pub struct Node {
    pub relay: Relay,
}

impl Node {

    #[tracing::instrument()]
    pub async fn debug_span_test(test_str: &str, empty_val: Option<String>) {
        // Span::current().
        Span::current().record("test_str", "asdf");
        tracing::info!("start services tracing node_id test2");
    }

    #[tracing::instrument(skip(relay))]
    pub async fn start_services(relay: Relay) -> Vec<JoinHandle<Result<(), ErrorInfo>>> {
        Span::current().record("node_id", &relay.node_config.public_key().short_id());
        tracing::info!("start services tracing node_id test");
        // Self::debug_span_test("input_func", None).await;
        let mut join_handles = vec![];
        let node_config = relay.node_config.clone();

        // let (p2p_rx_s, p2p_rx_r) = futures::channel::mpsc::channel::<Event>(1000);

        // Concurrent processes
        // let (p2p, jh_p2p) = P2P::new(relay.clone(), runtimes.p2p.clone(), p2p_rx_s);
        // join_handles.extend(jh_p2p);

        let jh_ctrl = control_api::ControlServer {
            relay: relay.clone(),
        }
            .start();

        join_handles.push(jh_ctrl);

        let peer_tx_jh = PeerOutgoingEventHandler::new(
            relay.clone(),
        );
        join_handles.push(peer_tx_jh);

        let tx_p_jh = TransactionProcessContext::new(
            relay.clone(),
        );
        join_handles.push(tx_p_jh);

        // info!("Before sleep");
        // let dur = tokio::time::Duration::from_secs(3);
        // tokio::time::sleep(dur).await;
        // info!("After sleep");


        // TODO: Monitor this join handle for errors.
        // runtimes
        //     .auxiliary
        //     .spawn(auto_update::from_node_config(node_config.clone()));
        //

        // Components for download now initialized.
        // relay.clone().node_state.store(NodeState::Downloading);


        let ojh = ObservationBuffer::new(relay.clone()).await;
        join_handles.push(ojh);

        // Rewards::new(relay.clone(), runtimes.auxiliary.clone());

        join_handles.push(PeerRxEventHandler::new(
            relay.clone(),
            // runtimes.auxiliary.clone(),
        ));

        join_handles.push(public_api::start_server(relay.clone(),
                                                   // runtimes.public_api.clone()
        ));

        join_handles.push(explorer::server::start_server(relay.clone(),
                                                   // runtimes.public_api.clone()
        ));

        let obs_handler = ObservationHandler{relay: relay.clone()};
        join_handles.push(tokio::spawn(async move { obs_handler.run().await }));
        //
        // let mut mph = MultipartyHandler::new(
        //     relay.clone(),
        //     // runtimes.auxiliary.clone()
        // );
        // join_handles.push(tokio::spawn(async move { mph.run().await }));

        let sm_port = relay.node_config.mparty_port();
        let sm_relay = relay.clone();
        join_handles.push(tokio::spawn(async move { gg20_sm_manager::run_server(sm_port, sm_relay)
                .await.map_err(|e| error_info(e.to_string())) }));


        // let relay_c = relay.clone();
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
        if node_config.e2e_enabled {
            // TODO: Distinguish errors here
            let _cwh = tokio::spawn(e2e::run(c_config));
            // join_handles.push(cwh);
        }

        join_handles.push(update_prometheus_configs(relay.clone()).await);

        let discovery = Discovery::new(relay.clone()).await;
        join_handles.push(stream_handlers::run_interval_fold(
            discovery.clone(), relay.node_config.discovery_interval, false
        ).await);

        join_handles.push(stream_handlers::run_interval_fold(
            Watcher::new(relay.clone()), relay.node_config.watcher_interval, false
        ).await);


        join_handles.push(stream_handlers::run_recv(
            discovery, relay.discovery.receiver.clone(), 100
        ).await);


        join_handles.push(tokio::spawn(api::rosetta::server::run_server(relay.clone())));

        join_handles.push(stream_handlers::run_interval_fold(
            Shuffle::new(&relay), relay.node_config.shuffle_interval, false
        ).await);

        join_handles
    }

    pub async fn prelim_setup(
        relay2: Relay,
        // runtimes: NodeRuntimes
    ) -> Result<(), ErrorInfo> {
        let relay = relay2.clone();
        let node_config = relay.node_config.clone();

        relay.ds.run_migrations_fallback_delete(
            node_config.clone().network != NetworkEnvironment::Main,
            node_config.env_data_folder().data_store_path()
        ).await?;
        relay.ds.count_gauges().await?;

        relay.ds.check_consistency_apply_fixes().await?;
        let prior_node_tx = relay.node_tx().await?;
        let nmd = prior_node_tx.node_metadata()?;
        let metadata = relay.node_config.node_metadata_fixed();
        if nmd != metadata {
            relay.update_node_metadata(&metadata).await?;
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

    pub fn genesis_from(node_config: NodeConfig) -> (Transaction, Vec<SpendableUTXO>) {
        let outputs = (0..50).map(|i|
            GenesisDistribution {
                address: node_config.internal_mnemonic().key_at(i as usize).address_typed(), amount: 10000
            }
        ).collect_vec();
        let tx = genesis_tx_from(outputs); //EARLIEST_TIME
        let res = tx.to_utxo_entries(EARLIEST_TIME as u64).iter().zip(0..50).map(|(o, i)| {
            let kp = node_config.internal_mnemonic().key_at(i as usize);
            let s = SpendableUTXO {
                utxo_entry: o.clone(),
                key_pair: kp,
            };
            s
        }).collect_vec();
        (tx, res)
    }

    pub async fn from_config(relay: Relay) -> Result<Node, ErrorInfo> {

        let node = Self {
            relay: relay.clone()
        };

        let node_config = relay.node_config.clone();

        if node_config.genesis {
            info!("Starting from genesis");
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
            //     info!("Genesis local test multiple kp");

            let existing = relay.ds.config_store.get_maybe_proto::<Transaction>("genesis").await?;

            if existing.is_none() {
                let tx = Node::genesis_from(node_config.clone()).0;
                // runtimes.auxiliary.block_on(
                relay.ds.config_store.store_proto("genesis", tx.clone()).await?;
                let _res_err =
                    // runtimes.auxiliary.block_on(
                    relay
                        .ds
                        .transaction_store
                        .insert_transaction(&tx.clone(), EARLIEST_TIME, true, None)
                        .await?;
                // }
                // .expect("Genesis inserted or already exists");
            }

        } else {

            info!("Starting from seed nodes");
            let seed = if node_config.main_stage_network() {
                info!("Querying LB for node info");
                let a =
                    // runtimes.auxiliary.block_on(
                    node_config.lb_client().about().await?;
                    // )?;
                let tx = a.latest_metadata.safe_get_msg("Missing latest metadata from seed node")?;
                let pd = tx.outputs.get(0).expect("a").data.as_ref().expect("d").peer_data.as_ref().expect("pd");
                let nmd = pd.node_metadata.get(0).expect("nmd");
                let _vec = nmd.public_key_bytes().expect("ok");
                let vec1 = pd.peer_id.safe_get()?.clone().peer_id.safe_bytes()?.clone();
                // TODO: Derive from NodeMetadata?
                Seed{
                    peer_id: Some(PeerId::from_bytes(vec1)),
                    trust: vec![TrustData::from_label(1.0)],
                    public_key: Some(nmd.public_key.safe_get_msg("Missing pk on about").cloned()?),
                    external_address: nmd.external_address.clone(),
                    port_offset: Some(nmd.port_offset.unwrap_or(node_config.network.default_port_offset() as i64) as u32),
                    environments: vec![node_config.network as i32],
                }
            } else {
                relay.node_config.seeds.get(0).unwrap().clone()

            };
            let port = seed.port_offset.unwrap() + 1;
            let client = PublicClient::from(seed.external_address.clone(), port as u16, Some(relay.clone()));
            info!("Querying with public client for node info again on: {} : {:?}", seed.external_address, port);
            let response = client.about().await?;
            let result = response.peer_node_info.safe_get()?;

            info!("Got LB node info {}, adding peer", result.json_or());

            // TODO: How do we handle situation where we get self peer id here other than an error?
            relay.ds.peer_store.add_peer_new(result, 1f64,
                                             &relay.node_config.public_key()).await?;

            info!("Added peer, attempting download");

            let x = result.latest_node_transaction.safe_get()?;
            let metadata = x.node_metadata()?;
            let pk1 = metadata.public_key.safe_get()?;

            // ensure peer added successfully
            let key = pk1.clone();
            let qry_result = relay.ds.peer_store.query_public_key_node(&key).await;
            qry_result.log_error().ok();
            let opt_info = qry_result.expect("query public key node");
            let pk_store = opt_info
                .expect("query public key node")
                .node_metadata().expect("node metadata")
                .public_key.expect("pk");
            assert_eq!(&pk_store, pk1);

            // This triggers peer exploration.
            tracing::info!("Attempting discovery process of peers on startup for node");
            let mut discovery = Discovery::new(relay.clone()).await;
            discovery.interval_fold().await?;
            // This was only immediate discovery of that node and is already covered
            // // TODO: Remove this in favor of discovery
            // relay.discovery.sender.send_err(
            //     DiscoveryMessage::new(metadata.clone(), result.dynamic_node_metadata.clone())
            // )?;

            tokio::time::sleep(Duration::from_secs(3)).await;
            info!("Now starting download after discovery has ran.");

            // TODO Change this invocation to an .into() in a non-schema key module
            download::download(
                relay.clone(),
                pk1.clone()
            ).await;
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
    // futures: Vec<JoinHandle<Result<(), ErrorInfo>>>
}

impl LocalTestNodeContext {
    async fn new(id: u16, random_port_offset: u16, seed: Option<Seed>) -> Self {
        let mut node_config = NodeConfig::from_test_id(&id);
        node_config.port_offset = random_port_offset;
        if id == 0 {
            node_config.genesis = true;
        }
        for x in seed {
            node_config.seeds = vec![x];
        }
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
    async fn new(
        // runtime: Arc<Runtime>,
        offset: Option<u16>) -> LocalNodes {
        let port_offset = offset.unwrap_or(util::random_port());
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
        let start = LocalTestNodeContext::new(0, port_offset, None).await;
        LocalNodes {
            current_seed: start.node.relay.node_config.self_seed(),
            nodes: vec![start],
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
            Some(self.current_seed.clone()),
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
    e2e_async().await.expect("");
    // let runtime = build_runtime(8, "e2e");
    // runtime.block_on(e2e_async()).expect("e2e");
}


async fn e2e_async() -> Result<(), ErrorInfo> {
    util::init_logger_once();
    metrics_registry::register_metric_names();
    // metrics_registry::init_print_logger();
    // init_tracing();
    let _tc = TestConstants::new();

    let mut local_nodes = LocalNodes::new(None).await;
    let start_node = local_nodes.start();
    // info!("Started initial node");
    let client1 = start_node.control_client.clone();
    // let ds = start_node.node.relay.ds.clone();
    // let show_balances = || {
    //     let res = &ds.query_all_balance();
    //     let res2 = res.as_ref().unwrap();
    //     let str = serde_json::to_string(&res2).unwrap();
    //     info!("Balances: {}", str);
    // };

    // show_balances();

    let client = start_node.public_client.clone();
    //
    // let utxos = ds.query_time_utxo(0, util::current_time_millis())
    //     .unwrap();
    // info!("Num utxos from genesis {:?}", utxos.len());

    let (_, spend_utxos) = Node::genesis_from(start_node.node.relay.node_config.clone());
    let submit = TransactionSubmitter::default(client.clone(),
                                               // runtime.clone(),
                                               spend_utxos
    );

    submit.submit().await.expect("submit");

    submit.submit_test_contract().await.expect("submit test contract");



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

    let do_signing = |party: ControlMultipartyKeygenResponse| async {

        let signing_data = Hash::from_string_calculate("hey");
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
            client1.multiparty_signing(signing_request).await;
        // println!("{:?}", res);
        assert!(res.is_ok());
        res.expect("ok").proof.expect("prof").verify(&signing_data).expect("verified");
    };

    do_signing(keygen1).await;

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
    do_signing(keygen2).await;


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
        relay.ds.transaction_store.insert_transaction(&tx, 0,true, None).await.expect("");
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
