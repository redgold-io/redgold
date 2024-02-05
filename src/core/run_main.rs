use std::env;
use clap::Parser;
use log::{error, info};
use metrics::counter;
use crate::node_config::NodeConfig;
use crate::util::cli::arg_parse_config;
use crate::util::cli::args::RgArgs;
use crate::util::metrics_registry;
use crate::util::runtimes::build_runtime;
use std::thread::sleep;
use std::time::Duration;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use redgold_schema::structs::ErrorInfo;
use crate::core::internal_message;
use crate::core::relay::Relay;
use crate::node::{Node};
use crate::util::cli::arg_parse_config::ArgTranslate;


pub async fn main_from_args(opts: RgArgs) {

    info!("Starting node main method");
    counter!("redgold.node.main_started").increment(1);

    let mut node_config = NodeConfig::default();

    let mut arg_translate = ArgTranslate::new(&opts, &node_config.clone());
    let _ = &arg_translate.translate_args().await.expect("arg translation");
    node_config = arg_translate.node_config.clone();

    tracing::info!("Starting network environment: {}", node_config.clone().network.to_std_string());

    if arg_translate.abort {
        return;
    }

    if arg_translate.is_gui() {
        crate::gui::initialize::attempt_start(node_config.clone()).await.expect("GUI to start");
        return;
    }

    let relay = Relay::new(node_config.clone()).await;

    Node::prelim_setup(relay.clone()).await.expect("prelim");

    // TODO: Tokio select better?
    let join_handles = Node::start_services(relay.clone()).await;
    let mut futures = FuturesUnordered::new();
    for jhi in join_handles {
        futures.push(jhi);
    }
    match Node::from_config(relay).await {
        Ok(_) => {
            info!("Node startup successful");
            match internal_message::map_fut(futures.next().await) {
                Ok(_) => {
                    error!("Some sub-service has terminated cleanly");
                }
                Err(e) => {
                    error!("Main service error: {}", crate::schema::json(&e).expect("json render of error failed?"));
                    panic!("Error in sub-service in main thread");
                }
            }
        }
        Err(e) => {
            error!("Node startup failure: {}", crate::schema::json(&e).expect("json render of error failed?"));
        }
    }
}

pub async fn main() {
    let opts = RgArgs::parse();
    main_from_args(opts).await;
}
