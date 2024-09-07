use std::env;
use std::thread::sleep;
use std::time::Duration;

use clap::Parser;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::{error, info};
use metrics::{counter, gauge};
use redgold_schema::SafeOption;
use redgold_schema::helpers::easy_json::{EasyJson, json_or};

use redgold_schema::structs::ErrorInfo;

use crate::core::internal_message;
use crate::core::relay::Relay;
use crate::integrations::external_network_resources::ExternalNetworkResourcesImpl;
use crate::node::Node;
use crate::node_config::NodeConfig;
use crate::util::cli::arg_parse_config;
use crate::util::cli::arg_parse_config::ArgTranslate;
use crate::util::cli::args::RgArgs;
use crate::util::runtimes::build_runtime;

pub async fn main_from_args(opts: RgArgs) {

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
        crate::gui::initialize::attempt_start(node_config.clone()).await.expect("GUI to start");
        return;
    }
    gauge!("redgold_service_crash", &node_config.gauge_id()).set(0);
    gauge!("redgold_start_fail", &node_config.gauge_id()).set(0);

    let relay = Relay::new(node_config.clone()).await;


    Node::prelim_setup(relay.clone()).await.expect("prelim");

    // TODO: Match on node_config external network resources impl
    // TODO: Tokio select better?
    let join_handles = Node::start_services(relay.clone(), ExternalNetworkResourcesImpl::new(&node_config).expect("works")).await;
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

pub async fn main() {
    let opts = RgArgs::parse();
    main_from_args(opts).await;
}
