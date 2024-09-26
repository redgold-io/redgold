use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use clap::Parser;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use tracing::{error, info};
use metrics::{counter, gauge};
use tokio::sync::Mutex;
use redgold::core::relay::Relay;
use redgold::gui;
use redgold::gui::native_gui_dependencies::NativeGuiDepends;
use redgold::integrations::external_network_resources::{ExternalNetworkResourcesImpl, MockExternalResources};
use redgold::node::Node;
use redgold::util::cli::arg_parse_config::ArgTranslate;
use redgold_schema::SafeOption;
use redgold_schema::helpers::easy_json::{EasyJson, json_or};

use redgold_schema::structs::ErrorInfo;

use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::conf::rg_args::RgArgs;

#[tokio::main]
async fn main() {
    let opts = RgArgs::parse();
    info!("Starting node main method");
    counter!("redgold.node.main_started").increment(1);

    let mut node_config = NodeConfig::default();

    let mut arg_translate = ArgTranslate::new(&opts, &node_config.clone());
    let _ = &arg_translate.translate_args().await.expect("arg translation");
    node_config = arg_translate.node_config.clone();

    tracing::trace!("Starting network environment: {}", node_config.clone().network.to_std_string());

    if arg_translate.abort {
        return;
    }

    if arg_translate.is_gui() {
        let res = ExternalNetworkResourcesImpl::new(&node_config).expect("works");
        let g = NativeGuiDepends::new(node_config.clone());
        gui::initialize::attempt_start(node_config.clone(), res, g).await.expect("GUI to start");
        return;
    }
    gauge!("redgold_service_crash", &node_config.gauge_id()).set(0);
    gauge!("redgold_start_fail", &node_config.gauge_id()).set(0);

    let mut relay = Relay::new(node_config.clone()).await;


    Node::prelim_setup(relay.clone()).await.expect("prelim");

    // TODO: Match on node_config external network resources impl
    // TODO: Tokio select better?
    let join_handles = if node_config.opts.debug_id.is_none() {
        Node::start_services(relay.clone(), ExternalNetworkResourcesImpl::new(&node_config).expect("works")).await
    } else {
        let dir = node_config.data_folder.path.join("mock_resources");
        let resources = MockExternalResources::new(&node_config, Some(dir), Arc::new(Mutex::new(HashMap::new()))).expect("works");
        relay.node_config.config_data.party_config_data.poll_interval = 10_000;
        Node::start_services(relay.clone(), resources).await
    };
    let mut futures = FuturesUnordered::new();
    for jhi in join_handles {
        futures.push(jhi.result())
    }
    let err = match Node::from_config(relay).await {
        Ok(n) => {
            info!("Node startup successful");
            let result = futures.next().await.ok_msg("No future result?").and_then(|e| e);
            let str_res = match result {
                Ok(e) => {
                    format!("Some sub-service has terminated cleanly {}", e.clone())
                }
                Err(e) => {
                    format!("Error in sub-service in main thread: {}", e.json_or())
                }
            };
            gauge!("redgold_service_crash", &node_config.gauge_id()).set(1.0);
            format!("Node service crash failure: {}", str_res)
        }
        Err(e) => {
            gauge!("redgold_start_fail", &node_config.gauge_id()).set(1.0);
            format!("Node startup failure: {}", e.json_or())
        }
    };
    error!("{}", err);
    panic!("{}", err);
}
