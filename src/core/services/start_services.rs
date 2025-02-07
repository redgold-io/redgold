use crate::api::udp_api::UdpServer;
use crate::api::udp_keepalive::UdpKeepAlive;
use crate::api::{control_api, explorer, public_api};
use crate::core::contract::contract_state_manager::ContractStateManager;
use crate::core::discover::data_discovery::DataDiscovery;
use crate::core::discover::peer_discovery::Discovery;
use crate::core::misc_periodic::MiscPeriodic;
use crate::core::observation::ObservationBuffer;
use crate::core::process_observation::ObservationHandler;
use crate::core::process_transaction::TransactionProcessContext;
use crate::core::relay::Relay;
use crate::core::services::service_join_handles::{NamedHandle, ServiceJoinHandles};
use crate::core::transact::contention_conflicts::ContentionConflictManager;
use crate::core::transact::tx_writer::TxWriter;
use crate::core::transport::peer_event_handler::PeerOutgoingEventHandler;
use crate::core::transport::peer_rx_event_handler::PeerRxEventHandler;
use crate::multiparty_gg20::gg20_sm_manager;
use crate::node::Node;
use crate::observability::dynamic_prometheus::update_prometheus_configs;
use crate::observability::metrics_registry;
use crate::party::party_watcher::PartyWatcher;
use crate::party::portfolio_fulfillment_agent::PortfolioFullfillmentAgent;
use crate::sanity::recent_parity::RecentParityCheck;
use crate::shuffle::shuffle_interval::Shuffle;
use crate::{api, e2e, node_config};
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_common_no_wasm::stream_handlers::{run_interval_fold, run_interval_fold_or_recv, run_recv_concurrent, run_recv_single};
use redgold_schema::error_info;
use std::time::Duration;

impl Node {
    /**
    * Start all background thread application services. REST APIs, event processors, transaction process, etc.
    * Each of these application background services communicates via channels instantiated by the relay
    TODO: Refactor this into multiple start routines, or otherwise delay service start until after preliminary setup.
    */
    #[tracing::instrument(skip(relay, external_network_resources), fields(node_id = %relay.node_config.public_key().short_id()
    ))]
    pub async fn start_services<T>(relay: Relay, external_network_resources: T) -> Vec<NamedHandle>
    where
        T: ExternalNetworkResources + Send + 'static + Sync + Clone
    {
        let node_config = relay.node_config.clone();

        if !node_config.disable_metrics {
            metrics_registry::register_metrics(node_config.port_offset);
        }


        let mut sjh = ServiceJoinHandles::default();

        if node_config.enable_party_mode() {
            sjh.add("CoinbaseWsStatus", 
            tokio::spawn(redgold_crawler_native::coinbase_ws::run_coinbase_ws_status(relay.coinbase_ws_status.sender.clone())));
        }

        let agent = PortfolioFullfillmentAgent::new(
            &relay, external_network_resources.clone());

        sjh.add("PortfolioFulfillmentAgent",
                run_interval_fold(agent, node_config.portfolio_fulfillment_agent_duration(),
                                  false));

        let udp = UdpServer::new(
            relay.peer_message_rx.clone(),
            relay.udp_outgoing_messages.clone(),
            Some(node_config.udp_port()));
        sjh.add("UdpServer", tokio::spawn(udp));

        if node_config.nat_traversal_required() ||
            relay.node_metadata().await.map(|n| n.nat_traversal_required()).unwrap_or(false) {
            let alive = UdpKeepAlive::new(&relay.peer_message_tx,
                                          node_config.udp_keepalive(),
                                          vec![],
                                          &relay
            );
            sjh.add("UdpKeepAlive", alive);
        };
        // Internal RPC control equivalent, used for issuing commands to node
        // Disabled in high security mode
        // Only needs to run on certain environments?
        sjh.add("ControlServer", control_api::ControlServer {
            relay: relay.clone(),
        }.start());
        // Stream processor for sending external peer messages
        // Negotiates appropriate protocol depending on peer
        sjh.add("PeerOutgoingEventHandler", PeerOutgoingEventHandler::new(relay.clone()));

        // Main transaction processing loop, watches over lifecycle of a given transaction
        // as it's drawn from the mem-pool
        sjh.add("TransactionProcessContext", TransactionProcessContext::new(relay.clone()));
        sjh.add("TxWriter", run_recv_single(TxWriter::new(&relay), relay.tx_writer.receiver.clone()).await);

        // TODO: Re-enable auto-update process for installed service as opposed to watchtower docker usage.
        // runtimes
        //     .auxiliary
        //     .spawn(auto_update::from_node_config(node_config.clone());
        // Components for download now initialized.
        // relay.clone().node_state.store(NodeState::Downloading);

        let ojh = ObservationBuffer::new(relay.clone()).await;
        sjh.add("ObservationBuffer", ojh);

        sjh.add("PeerRxEventHandler", PeerRxEventHandler::new(
            relay.clone(),
        ));

        sjh.add("public_api", public_api::start_server(relay.clone()));

        sjh.add("explorer", explorer::server::start_server(relay.clone()));

        let obs_handler = ObservationHandler { relay: relay.clone() };
        sjh.add("obs_handler", tokio::spawn(async move { obs_handler.run().await }));

        let sm_port = relay.node_config.mparty_port();
        let sm_relay = relay.clone();
        sjh.add("gg20_sm_manager", tokio::spawn(async move {
            gg20_sm_manager::run_server(sm_port, sm_relay)
                .await.map_err(|e| error_info(e.to_string()))
        }));

        let c_config = relay.clone();
        if node_config.e2e_enabled() {
            // TODO: Distinguish errors here
            let cwh = tokio::spawn(e2e::run(c_config));
            sjh.add("e2e", cwh);
        }

        sjh.add("update_prometheus_configs", update_prometheus_configs(relay.clone()).await);

        let discovery = Discovery::new(relay.clone()).await;
        sjh.add("discovery.interval", run_interval_fold(
            discovery.clone(), relay.node_config.discovery_interval, false
        ));


        sjh.add("discovery.receiver", run_recv_concurrent(
            discovery, relay.discovery.receiver.clone(), 100
        ).await);

        let watcher = PartyWatcher::new(&relay, external_network_resources);
        sjh.add("PartyWatcher", run_interval_fold(
            watcher, relay.node_config.party_poll_interval(), false
        ));

        sjh.add("rosetta", tokio::spawn(api::rosetta::server::run_server(relay.clone())));

        sjh.add("Shuffle", run_interval_fold(
            Shuffle::new(&relay), relay.node_config.shuffle_interval, false
        ));

        sjh.add("Mempool", run_interval_fold(
            crate::core::mempool::Mempool::new(&relay), relay.node_config.mempool.interval.clone(), false
        ));

        for i in 0..relay.node_config.contract.bucket_parallelism {
            let opt_c = relay.contract_state_manager_channels.get(i);
            let c = opt_c.expect("bucket partition creation error");
            let handle = run_interval_fold_or_recv(
                ContractStateManager::new(relay.clone()),
                relay.node_config.contract.interval.clone(),
                false,
                c.receiver.clone()
            ).await;
            sjh.add("ContractStateManager", handle);

            let opt_c = relay.contention.get(i);
            let c = opt_c.expect("bucket partition creation error");
            let handle = run_interval_fold_or_recv(
                ContentionConflictManager::new(relay.clone()),
                relay.node_config.contention.interval.clone(),
                false,
                c.receiver.clone()
            ).await;
            sjh.add("ContentionConflictManager", handle);
        }

        sjh.add("DataDiscovery", run_interval_fold(
            DataDiscovery {
                relay: relay.clone(),
            }, Duration::from_secs(300), false
        ));

        sjh.add("MiscPeriodic", run_interval_fold(
            MiscPeriodic::new(&relay), Duration::from_secs(300), false
        ));
        sjh.add("AwsBackup", run_interval_fold(
            crate::core::backup::aws_backup::AwsBackup::new(&relay), Duration::from_secs(86400), false
        ));
        sjh.add("RecentParityCheck", run_interval_fold(
            RecentParityCheck::new(&relay), Duration::from_secs(3600), false
        ));

        if let Some(jh) = relay.eth_daq.start(&relay.node_config).await {
            let result = jh.expect("eth_daq start failed");
            sjh.add("EthDaq", result);
        }

        sjh.handles
    }
}