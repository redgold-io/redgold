use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use futures::{stream, StreamExt};
use futures::stream::FuturesUnordered;
use itertools::Itertools;
use log::error;
use tracing::info;
use metrics::{counter, gauge};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use redgold_schema::constants::REWARD_AMOUNT;
use redgold_schema::{bytes_data, error_info, structs, ErrorInfoContext, RgResult, SafeOption, ShortString};
use redgold_schema::structs::{ControlMultipartyKeygenResponse, ControlMultipartySigningRequest, CurrencyAmount, GetPeersInfoRequest, Hash, InitiateMultipartySigningRequest, NetworkEnvironment, PeerId, PeerNodeInfo, Request, Seed, State, TestContractInternalState, Transaction, TrustData, ValidationType};
use crate::core::transact::tx_writer::TxWriter;
use crate::api::control_api::ControlClient;
// use crate::api::p2p_io::rgnetwork::Event;
// use crate::api::p2p_io::P2P;
use crate::api::{explorer, public_api};
use crate::api::client::public_client::PublicClient;
use crate::api::{control_api, rosetta};
use crate::e2e::tx_submit::TransactionSubmitter;
use crate::core::{block_formation, stream_handlers};
use crate::core::observation::ObservationBuffer;
use crate::core::transport::peer_event_handler::PeerOutgoingEventHandler;
use crate::core::transport::peer_rx_event_handler::PeerRxEventHandler;
use crate::core::process_transaction::TransactionProcessContext;
use crate::core::relay::Relay;
use redgold_data::data_store::DataStore;
use crate::data::download;
use crate::genesis::{genesis_transaction, genesis_tx_from, GenesisDistribution};
use redgold_schema::conf::node_config::NodeConfig;
use crate::schema::structs::{ControlRequest, ErrorInfo, NodeState};
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
// use crate::trust::rewards::Rewards;
use crate::{api, e2e, util};
// use crate::mparty::mp_server::{Db, MultipartyHandler};
use crate::e2e::tx_gen::SpendableUTXO;
use crate::core::process_observation::ObservationHandler;
use crate::multiparty_gg20::gg20_sm_manager;
use crate::util::runtimes::build_runtime;
use crate::util::{auto_update, keys};
use crate::schema::constants::EARLIEST_TIME;
use redgold_keys::TestConstants;
use tokio::task::spawn_blocking;
use tracing::{trace, Span};
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_keys::proof_support::ProofSupport;
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::TransactionState::Mempool;
use crate::api::rosetta::models::Peer;
use crate::core::contract::contract_state_manager::ContractStateManager;
use crate::core::discover::data_discovery::DataDiscovery;
use crate::core::discover::peer_discovery::{Discovery, DiscoveryMessage};
use redgold_common::flume_send_help::SendErrorInfo;
use crate::core::recent_download::RecentDownload;
use crate::core::stream_handlers::{run_recv_single, IntervalFold};
use crate::core::transact::contention_conflicts::ContentionConflictManager;
use crate::infra::multiparty_backup::check_updated_multiparty_csv;
use crate::multiparty_gg20::initiate_mp::default_room_id_signing;
// use crate::multiparty_gg20::watcher::DepositWatcher;
use crate::observability::dynamic_prometheus::update_prometheus_configs;
use crate::shuffle::shuffle_interval::Shuffle;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::proto_serde::{ProtoHashable, ProtoSerde};
use redgold_schema::util::lang_util::WithMaxLengthString;
use crate::core::misc_periodic::MiscPeriodic;
use crate::core::services::service_join_handles::{NamedHandle, ServiceJoinHandles};
use crate::integrations::external_network_resources::{ExternalNetworkResourcesImpl, MockExternalResources};
use redgold_keys::word_pass_support::WordsPassNodeConfig;
// use crate::api::p2p_io::rgnetwork::Event;
// use crate::api::p2p_io::P2P;
use crate::api::client::rest::RgHttpClient;
use crate::party::party_watcher::PartyWatcher;
use crate::sanity::{historical_parity, migrations};
use crate::sanity::recent_parity::RecentParityCheck;

/**
* Node is the main entry point for the application /
* blockchain node runtime.
* It is responsible for starting all the services and
* Initializing the connection to the network
* managing the lifecycle of the application.
*/
#[derive(Clone)]
pub struct Node {
    pub relay: Relay,
}
impl Node {

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

        migrations::apply_migrations(&relay).await
            .log_error()
            .add("Historical parity manual migration failed")?;

        check_updated_multiparty_csv(&relay).await
            .add("Multiparty CSV update failed")
            .log_error()?;

        relay.ds.peer_store.clear_all_peers().await?;

        if let Some(p) = fs::read_to_string(relay2.node_config.env_data_folder().peer_tx_path()).ok() {
            if let Ok(tx) = p.json_from::<Transaction>() {
                relay.ds.config_store.set_peer_tx(&tx).await?;
                let pd = tx.peer_data()?;
                let key = node_config.public_key();
                let self_pk = Some(key);
                let nmd = pd.node_metadata.iter()
                    .filter(|x| { x.public_key == self_pk }).next().cloned().ok_msg("No node metadata found")?;
                let nmd = relay2.update_with_live_info(nmd).await?;
                relay2.update_node_metadata(&nmd).await?;
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

    pub fn genesis_from(node_config: NodeConfig) -> (Transaction, Vec<SpendableUTXO>) {
        let tx = genesis_transaction(&node_config, &node_config.words(), &node_config.seeds_now());
        let outputs = tx.utxo_outputs().expect("utxos");
        let mut res = vec![];
        for i in 0..50 {
            let kp = node_config.words().keypair_at_change(i).expect("works");
            let address = kp.address_typed();
            let o = outputs.iter().find(|o| {
                address == o.address().as_ref().expect("a").clone().clone()
            }).expect("found");
            let s = SpendableUTXO {
                utxo_entry: o.clone(),
                key_pair: kp,
            };
            res.push(s);
        }
        (tx, res)
    }

    pub async fn from_config(relay: Relay) -> Result<Node, ErrorInfo> {

        let node = Self {
            relay: relay.clone()
        };

        let node_config = relay.node_config.clone();

        // relay.force_update_nmd_auto_peer_tx().await?;

        // Temp mechanism, clear all peers, allow seeds to refresh them.

        // relay.update_nmd_auto().await?;

        gauge!("redgold_peer_id", &relay.gauge_labels().await?).set(1.0);

        if node_config.genesis() {
            Self::genesis_start(&relay, &node_config).await?;
        } else {

            info!("Starting from seed nodes");

            let all_seeds = if node_config.main_stage_network() {
                relay.node_config.seeds_now().clone()
            } else {
                vec![relay.node_config.seeds_now().get(0).unwrap().clone()]
            };

            let seeds = relay.node_config.non_self_seeds();


            let seed_results = if node_config.config_data.debug.as_ref().and_then(
                |c| c.bypass_seed_enrichment.clone()).unwrap_or(false) {
                vec![]
            } else{
                        stream::iter(seeds)
                            .then(|seed| {
                                Self::query_seed(&relay, &node_config, seed)
                            })
                            .filter_map(|x| async {
                                let res = x.ok();
                                if res.is_none() {
                                    counter!("redgold_node_seed_startup_query_failure").increment(1);
                                }
                                res
                            })
                            .collect::<Vec<PeerNodeInfo>>()
                            .await
                    };

            let bootstrap_pks = seed_results.iter().filter_map(|x| {
                x.nmd_pk()
            }).collect_vec();

            gauge!("redgold.node.seed_startup_query_count").set(seed_results.len() as f64);

            // TODO: Allow bypassing this check for testing purposes
            if seed_results.is_empty() {
                if node_config.is_self_seed() {
                    error!("No seed nodes found, but this is a self seed node, continuing");
                } else {
                    return Err(error_info("No seed nodes found, exiting"));
                }
            }

            for result in &seed_results {
                relay.ds.peer_store.add_peer_new(result, &relay.node_config.public_key()).await?;
            }
            info!("Added seed peers, attempting download");

            // This triggers peer exploration immediately
            tracing::info!("Attempting discovery process of peers on startup for node");
            let mut discovery = Discovery::new(relay.clone()).await;
            discovery.interval_fold().await?;

            tokio::time::sleep(Duration::from_secs(3)).await;
            info!("Now starting download after discovery has run.");
            // TODO Change this invocation to an .into() in a non-schema key module
            if node_config.config_data.debug.as_ref().and_then(
                |c| c.bypass_download.clone()).unwrap_or(false) {
                info!("Bypassing download");
            } else {
                match download::download(
                    relay.clone(), bootstrap_pks
                ).await.log_error() {
                    Ok(_) => {}
                    Err(e) => {
                        if node_config.is_self_seed() {
                            error!("Download failed, but this is a self seed node, continuing");
                        } else {
                            return Err(e);
                        }
                    }
                }
            }


        }

        trace!("Node ready");
        counter!("redgold.node.node_started").increment(1);

        // gauge!("redgold.node.node_started").set(1.0);
        let gen_tx = relay.ds.config_store.get_genesis()
            .await.expect("Genesis query fail").expect("genesis hash missing")
            .hash_hex()
            .short_string()
            .expect("genesis hash short string");
        let id = relay.node_config.gauge_id().to_vec();
        let labels = [("genesis_hash".to_string(), gen_tx), id.get(0).cloned().expect("id")];
        gauge!("redgold.node.genesis_hash", &labels).set(1.0);
        return Ok(node);
    }

    async fn query_seed(relay: &Relay, node_config: &NodeConfig, seed: Seed) -> Result<PeerNodeInfo, ErrorInfo> {
        let api_port = seed.port_or(node_config.port_offset) + 1;
        let client = PublicClient::from(seed.external_address.clone(), api_port, Some(relay.clone()));
        info!("Querying with public client for node info again on: {} : {:?}", seed.external_address, api_port);
        let response = client.about().await?;
        let result = response.peer_node_info.safe_get()?;
        Ok(result.clone())
    }

    async fn genesis_start(relay: &Relay, node_config: &NodeConfig) -> Result<(), ErrorInfo> {
        // info!("Starting from genesis");
        let existing = relay.ds.config_store.get_maybe_proto::<Transaction>("genesis").await?;

        if existing.is_none() {
            counter!("redgold.node.genesis_created").increment(1);
            // info!("No genesis transaction found, generating new one");
            let tx = genesis_transaction(&node_config, &node_config.words(), &node_config.seeds_now());
            // info!("Genesis transaction generated {}", tx.json_or());
            // let tx = Node::genesis_from(node_config.clone()).0;
            // runtimes.auxiliary.block_on(
            relay.ds.config_store.store_proto("genesis", tx.clone()).await?;
            let _res_err =
                // runtimes.auxiliary.block_on(
                relay
                    .write_transaction(&tx.clone(), EARLIEST_TIME, None, true)
                    .await.expect("insert failed");
            // }
            let genesis_hash = tx.hash_or();
            // info!("Genesis hash {}", genesis_hash.hex());
            let _obs = relay.observe_tx(&genesis_hash, State::Pending, ValidationType::Full, structs::ValidationLiveness::Live).await?;
            let _obs = relay.observe_tx(&genesis_hash, State::Accepted, ValidationType::Full, structs::ValidationLiveness::Live).await?;
            assert_eq!(relay.ds.observation.select_observation_edge(&genesis_hash).await?.len(), 2);
        }
        Ok(())
    }
}
