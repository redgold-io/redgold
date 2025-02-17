use crate::api::client::public_client::PublicClient;
use redgold_common::client::http::RgHttpClient;
use crate::api::control_api::ControlClient;
use crate::core::relay::Relay;
use crate::e2e::tx_submit::TransactionSubmitter;
use crate::integrations::external_network_resources::MockExternalResources;
use crate::node::Node;
use crate::util;
use crate::util::runtimes::big_thread;
use itertools::Itertools;
use redgold_keys::word_pass_support::WordsPassNodeConfig;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::proto_serde::{ProtoHashable, ProtoSerde};
use redgold_schema::structs::{ErrorInfo, Seed, SupportedCurrency, Transaction};
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use redgold_schema::{structs, SafeOption};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Clone)]
pub struct LocalTestNodeContext {
    id: u16,
    port_offset: u16,
    pub(crate) node: Node,
    pub(crate) public_client: PublicClient,
    pub(crate) control_client: ControlClient,
    // futures: Vec<JoinHandle<Result<(), ErrorInfo>>>
}

impl LocalTestNodeContext {
    async fn new(id: u16, random_port_offset: u16, seed: Vec<Seed>,
                 ext: Arc<Mutex<HashMap<SupportedCurrency, Vec<ExternalTimedTransaction>>>>,
    ) -> Self {

        let nc = big_thread().spawn(move || {
            Self::get_node_config(&id, random_port_offset, seed.clone())
        });
        let node_config = nc.unwrap().join().unwrap();

        // let runtimes = NodeRuntimes::default();
        let relay = Relay::new(node_config.clone()).await;
        Node::prelim_setup(relay.clone()
                           // , runtimes.clone()
        ).await.expect("prelim");


        // info!("Test starting node services");
        // info!("Test starting node services for node id {id}");

        let resources = MockExternalResources::new(&node_config, None, ext).expect("works");
        let ext = resources.external_transactions.clone();
        let futures = Node::start_services(relay.clone(), resources).await;

        // info!("Test completed starting node services for node id {id}");
        tokio::spawn(async move {
            // TODO: Get the join errors here
            let mut fut2 = vec![];
            for f in futures {
                fut2.push(Box::pin(f.result()));
            }
            let (res, _, _) = futures::future::select_all(fut2).await;
            let result = res.log_error();
            panic!("Node service failed in test: {:?}", result);
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
            public_client: PublicClient::local(node.relay.node_config.public_port(), Some(Box::new(relay.clone()))),
            control_client: ControlClient::local(node.relay.node_config.control_port()),
            // futures
        }
    }

    fn get_node_config(id: &u16, random_port_offset: u16, seed: Vec<Seed>) -> NodeConfig {
        let mut node_config = NodeConfig::from_test_id(&id);
        let mut pd = (*node_config.config_data).clone();
        let mut party_data = pd.party.unwrap_or_default();
        party_data.order_cutoff_delay_time = Some(5_000);
        party_data.poll_interval = Some(5_000);
        pd.party = Some(party_data);
        node_config.config_data = Arc::new(pd);
        node_config.port_offset = random_port_offset;
        if *id == 0 {
            let mut opts = (&*node_config.config_data.clone()).clone();
            let mut p = opts.party.clone().unwrap_or_default();
            opts.debug.get_or_insert(Default::default()).genesis = Some(true);
            p.enable = Some(true);
            opts.party = Some(p);
            node_config.config_data = Arc::new(opts);
        }
        node_config.seeds = seed.clone();
        node_config
    }
}

async fn throw_panic() {
    panic!("that happened");
}

pub struct LocalNodes {
    pub nodes: Vec<LocalTestNodeContext>,
    current_seed: Seed,
    pub(crate) seeds: Vec<Seed>,
    pub ext: Arc<Mutex<HashMap<SupportedCurrency, Vec<ExternalTimedTransaction>>>>,
    pub submit: TransactionSubmitter,
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

    pub(crate) fn start(&self) -> &LocalTestNodeContext {
        self.nodes.get(0).unwrap()
    }
    fn current_seed_id(&self) -> u16 {
        self.nodes.len() as u16
    }
    fn seeds() -> Vec<Seed> {
        let mut seeds = vec![];
        for idx in 0..3 {
            let mut seed = NodeConfig::from_test_id(&idx).self_seed();
            seed.port_offset = Some(util::random_port() as u32);
            seeds.push(seed);
        }
        seeds
    }

    pub(crate) async fn new(
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
        let arc = Arc::new(Mutex::new(HashMap::new()));
        let start = LocalTestNodeContext::new(0, s, Self::seeds(), arc.clone()).await;

        let vec = start.node.relay.ds.utxo.utxo_all_debug().await.expect("utxo all debug");
        assert!(vec.len() > 0);
    
        let (_, spend_utxos) = Node::genesis_from(start.node.relay.node_config.clone());
    
        let submit = TransactionSubmitter::default(
            start.public_client.clone(), spend_utxos, &start.node.relay.node_config
        );

        let l = LocalNodes {
            current_seed: start.node.relay.node_config.self_seed(),
            nodes: vec![start.clone()],
            seeds,
            ext: arc,
            submit,
        };

    

        l
    }

    pub fn start_node(&self) -> &LocalTestNodeContext {
        self.nodes.get(0).unwrap()  
    }

    pub fn client(&self) -> RgHttpClient {
        self.nodes.get(0).unwrap().public_client.client_wrapper().clone()
    }
    
    pub fn public_client(&self) -> PublicClient {
        self.nodes.get(0).unwrap().public_client.clone()
    }
    
    pub fn config(&self) -> NodeConfig {
        self.nodes.get(0).unwrap().node.relay.node_config.clone()
    }

    pub fn clients(&self) -> Vec<RgHttpClient> {
        self.nodes.iter().map(|x| x.public_client.client_wrapper().clone()).collect_vec()
    }

    pub(crate) async fn verify_peers(&self) -> Result<(), ErrorInfo> {
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

    pub(crate) async fn verify_data_equivalent(&self) {
        let mut txs: Vec<HashSet<Transaction>> = vec![];
        let mut obs: Vec<HashSet<Vec<u8>>> = vec![];
        let mut oes: Vec<HashSet<Vec<Vec<u8>>>> = vec![];
        let mut utxos: Vec<HashSet<Vec<u8>>> = vec![];
        let end_time = util::current_time_millis();
        for n in &self.nodes {
            let tx: HashSet<Transaction> = n
                .node
                .relay
                .ds
                .transaction_store
                .query_time_transaction(0, end_time as i64).await
                .unwrap()
                .into_iter()
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
                .utxo
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
        Self::diff_check(&mut txs);
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

    fn diff_check<T: Serialize + Clone + PartialEq + Eq + std::hash::Hash>(txs: &mut Vec<HashSet<T>>) {
        let node_0_set = txs.get(0).unwrap().clone();
        for x in txs.clone() {
            let diff1 = node_0_set.difference(&x).collect_vec();
            let diff2 = x.difference(&node_0_set).collect_vec();
            let x2 = diff1.len() > 0 || diff2.len() > 0;
            if x2 {
                info!("Difference found in verify");
                info!("Diff1: {}", diff1.json_or());
                info!("Diff2: {}", diff2.json_or());
                assert!(false);
            }
        }
    }

    pub(crate) async fn add_node(&mut self) {
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
            self.ext.clone()
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
