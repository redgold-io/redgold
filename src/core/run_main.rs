use std::env;
use clap::Parser;
use log::{error, info};
use metrics::increment_counter;
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
use crate::core::internal_message::FutLoopPoll;
use crate::core::relay::Relay;
use crate::node::{Node, NodeRuntimes};
use crate::util::cli::arg_parse_config::ArgTranslate;


pub async fn main_from_args(opts: RgArgs) {
    // std::env::args() and ArgTranslate
    // ArgTranslate::new(opts).run();

    let mut node_config = NodeConfig::default();
    // let simple_runtime = build_runtime(1, "main");

    // TODO: Fix, borrowed node config here cannot be used to build the arg translate
    let mut arg_translate = ArgTranslate::new(
        // simple_runtime.clone(),
        &opts, node_config.clone());
    &arg_translate.run().expect("arg translation");
    node_config = arg_translate.node_config.clone();
    node_config = arg_parse_config::load_node_config_initial(opts.clone(), node_config);


    /// Commands required to run separate from the logging system initialization
    /// TODO: Consider disabling logging for these commands and instead unifying them?
    if arg_parse_config::immediate_commands(&opts, &node_config,
                                            // simple_runtime.clone()
    ).await {
        return;
    }
    // TODO: Change the port here by first parsing args associated with metrics / logs
    crate::util::init_logger_with_config(node_config.clone()).expect("Logger to start properly");
    metrics_registry::register_metrics(node_config.port_offset);

    info!("Starting node main method");
    increment_counter!("redgold.node.main_started");

    let node_config_res = arg_parse_config::load_node_config(
        // simple_runtime.clone(),
        opts.clone(), node_config
    ).await;


    // TODO: Here is where we should later init loggers and metrics?
    // and then build out the data store etc. ?
    match node_config_res {
        Ok(node_config) => {
            if arg_translate.is_gui() {
                crate::gui::initialize::attempt_start(node_config.clone()
                                                      // , simple_runtime.clone()
                )
                    .await
                    .expect("GUI to start");
                return;
            }
            let mut arg_translate = ArgTranslate::new(
                // simple_runtime.clone(),
                &opts, node_config.clone()
            );
            //simple_runtime.block_on(
            if arg_translate.post_logger_commands().await.expect("post logger commands") {
                return;
            }

            // let runtimes = NodeRuntimes::default(); simple_runtime.block_on(
            let mut relay = Relay::new(node_config.clone()).await;

            Node::prelim_setup(relay.clone(),
                               // runtimes.clone()
            ).await.expect("prelim");
            let mut join_handles = Node::start_services(relay.clone()
                                                        // , runtimes.clone()
            ).await;
            let mut futures = FuturesUnordered::new();
            for jhi in join_handles {
                futures.push(jhi);
            }
            let res = Node::from_config(relay
                                        // , runtimes
            ).await;
            match res {
                Ok(_) => {
                    info!("Node startup successful");
                    // loop {
                        match FutLoopPoll::map_fut(futures.next().await) {
                            Ok(_) => {
                                error!("Some sub-service has terminated cleanly");
                            }
                            Err(e) => {
                                error!("Main service error: {}", crate::schema::json(&e).expect("json render of error failed?"));
                                panic!("Error in sub-service in main thread");
                            }
                        }
                    // }
                }
                Err(e) => {
                    error!("Node startup failure: {}", crate::schema::json(&e).expect("json render of error failed?"));
                }
            }
            ();
        }
        Err(_) => {
            info!("Not starting node");
        }
    }
}

pub async fn main() {
    let opts = RgArgs::parse();
    main_from_args(opts).await;
}
