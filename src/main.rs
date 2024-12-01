use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use clap::Parser;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use itertools::Itertools;
use tracing::{error, info};
use metrics::{counter, gauge};
use tokio::runtime::Builder;
use tokio::sync::Mutex;
use redgold::core::relay::Relay;
use redgold::gui;
use redgold::gui::native_gui_dependencies::NativeGuiDepends;
use redgold::integrations::external_network_resources::{ExternalNetworkResourcesImpl, MockExternalResources};
use redgold::node::Node;
use redgold::node_config::ApiNodeConfig;
use redgold::util::cli::arg_parse_config::ArgTranslate;
use redgold::util::cli::immediate_commands;
use redgold::util::cli::load_config::{load_full_config, main_config};
use redgold_schema::SafeOption;
use redgold_schema::helpers::easy_json::{EasyJson, json_or};

use redgold_schema::structs::ErrorInfo;

use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::conf::rg_args::RgArgs;
use redgold_schema::observability::errors::Loggable;

async fn load_configs() -> (Box<NodeConfig>, bool) {
    let nc = main_config();
    let opts = RgArgs::parse();
    let cmd = opts.subcmd.as_ref().map(|x| Box::new(x.clone()));
    let mut arg_translate = Box::new(ArgTranslate::new(nc, opts));
    let nc = arg_translate.translate_args().await.expect("arg translation");
    let abort = immediate_commands::immediate_commands(
        &nc, cmd).await;
    (nc, abort)
}

// Stack debugging here.
// #[global_allocator]
// static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    // let _profiler = dhat::Profiler::new_heap();

    let runtime = Builder::new_multi_thread()
        .thread_stack_size(1024 * 1024 * 1024) // 1024 stack
        .worker_threads(num_cpus::get())  // Use all available logical cores
        .enable_all()
        .enable_time()
        .build()
        .unwrap();

    runtime.block_on(main_dbg());

}

// #[tokio::main]
// async fn main() {
async fn main_dbg() {

    let (node_config, abort) = load_configs().await;
    if abort {
        return
    }

    // info!("Starting node main method");
    counter!("redgold.node.main_started").increment(1);

    tracing::trace!("Starting network environment: {}", node_config.clone().network.to_std_string());

    if node_config.abort {
        return;
    }


    if node_config.is_gui {
        // this is a lot of data, only reason it's being preloaded here is due to stack.
        let party_data = if node_config.offline() {
            Default::default()
        } else {
            node_config.api_rg_client().enriched_party_data().await
        };
        let res = Box::new(ExternalNetworkResourcesImpl::new(&node_config, None).expect("works"));
        let g = Box::new(NativeGuiDepends::new(*node_config.clone()));
        gui::initialize::attempt_start(node_config, res, g, party_data).await.expect("GUI to start");
        return;
    }


    gauge!("redgold_service_crash", &node_config.gauge_id()).set(0);
    gauge!("redgold_start_fail", &node_config.gauge_id()).set(0);

    let relay = Relay::new(*node_config.clone()).await;


    Node::prelim_setup(relay.clone()).await.expect("prelim");

    // TODO: Match on node_config external network resources impl
    // TODO: Tokio select better?
    let join_handles = if node_config.debug_id().is_none() {
        Node::start_services(relay.clone(), ExternalNetworkResourcesImpl::new(&node_config, Some(relay.clone())).expect("works")).await
    } else {
        let dir = node_config.data_folder.path.join("mock_resources");
        let resources = MockExternalResources::new(&node_config, Some(dir), Arc::new(Mutex::new(HashMap::new()))).expect("works");
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
