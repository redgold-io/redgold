use std::collections::HashMap;
use std::{env, thread};
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
use redgold::gui::ClientApp;
use redgold::gui::initialize::start_native_gui;
use redgold::gui::native_gui_dependencies::NativeGuiDepends;
use redgold::integrations::external_network_resources::{ExternalNetworkResourcesImpl, MockExternalResources};
use redgold::node::Node;
use redgold::node_config::ApiNodeConfig;
use redgold::util::cli::arg_parse_config::ArgTranslate;
use redgold::util::cli::{commands, immediate_commands};
use redgold::util::cli::load_config::{load_full_config, main_config};
use redgold::util::runtimes::build_simple_runtime;
use redgold_gui::dependencies::gui_depends::GuiDepends;
use redgold_schema::{ErrorInfoContext, RgResult, SafeOption};
use redgold_schema::helpers::easy_json::{EasyJson, json_or};

use redgold_schema::structs::ErrorInfo;

use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::conf::rg_args::{RgArgs, RgTopLevelSubcommand};
use redgold_schema::config_data::ConfigData;
use redgold_schema::observability::errors::Loggable;

async fn load_configs() -> (Box<NodeConfig>, bool) {
    let (nc, opts) = main_config();
    // println!("loaded main config");
    let cmd = opts.subcmd.as_ref().map(|x| Box::new(x.clone()));
    let mut arg_translate = Box::new(ArgTranslate::new(nc, &opts));
    let nc = arg_translate.translate_args().await.expect("arg translation");
    // println!("translated args");
    // info!("Loaded config: {}", nc.config_data.json_or());
    let mut abort = false;
    if let Some(cmd) = cmd {
        match *cmd {
            RgTopLevelSubcommand::Node(_) => {}
            RgTopLevelSubcommand::GUI(_) => {}
            _ => {
                abort = true;
            }
        }
        match *cmd {
            RgTopLevelSubcommand::Balance(a) => {
                commands::balance_lookup(&a, &nc).await.unwrap();
            }
            RgTopLevelSubcommand::Address(a) => {
                commands::generate_address(a.clone(), &nc).map(|_| ()).unwrap();

            }
            RgTopLevelSubcommand::Send(a) => {
                commands::send(&a, &nc).await.unwrap();
            }
            RgTopLevelSubcommand::Query(a) => {
                commands::query(&a, &nc).await.unwrap();
            }
            RgTopLevelSubcommand::Deploy(d) => {
                commands::deploy(&d, &nc).await.unwrap().abort();
            }
            RgTopLevelSubcommand::DebugCommand(d) => {
                commands::debug_commands(&d, &nc).await.unwrap();
            }
            RgTopLevelSubcommand::GenerateConfig(d) => {
                let c = ConfigData::generate_user_sample_config();
                toml::to_string(&c)
                    .error_info("Failed to serialize config")
                    .map(|x| println!("{}", x))
                    .unwrap();
            }
            RgTopLevelSubcommand::GenerateRandomWords(a) => {
                // todo; from hardware entropy.
                let w = commands::generate_random_mnemonic().words;
                println!("{}", w);
            }
            _ => {}
        }
    }
    (nc, abort)
}
//
// Stack debugging here.
// #[global_allocator]
// static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    // println!("main");
    // let _profiler = dhat::Profiler::new_heap();

    // This is a workaround for the stack size issue inherit in clap opt parser
    // + large config classes.
    // All of these runtimes are just immediately discarded and only really
    // used for the CLI + gui issues
    // None of this is used for main node thread

    let (node_config, abort) = big_thread().spawn(|| {
        let runtime = build_simple_runtime(num_cpus::get(), "config");
        let ret = runtime.block_on(load_configs());
        runtime.shutdown_background();
        ret
    }).unwrap().join().unwrap();

    if abort {
        return
    }

    if node_config.is_gui {
        let gui = big_thread().spawn(|| {
            let runtime = build_simple_runtime(num_cpus::get(), "gui");
            let ret = runtime.block_on(gui_init(node_config));
            runtime.shutdown_background();
            ret
        }).unwrap().join().unwrap();
        let runtime = build_simple_runtime(num_cpus::get(), "gui");
        let ret = runtime.block_on(start_native_gui(gui)).expect("GUI to start");
        return
    }

    let ret = big_thread().spawn(|| {
        let runtime = build_simple_runtime(num_cpus::get(), "main");
        let ret = runtime.block_on(main_dbg(node_config));
        runtime.shutdown_background();
        ret
    }).unwrap().join().unwrap();


}

fn big_thread() -> std::thread::Builder {
    thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
}

async fn gui_init(node_config: Box<NodeConfig>) -> ClientApp<NativeGuiDepends> {
    // this is a lot of data, only reason it's being preloaded here is due to stack.
    let party_data = if node_config.offline() {
        Default::default()
    } else {
        node_config.api_rg_client().enriched_party_data().await
    };
    let res = Box::new(ExternalNetworkResourcesImpl::new(&node_config, None).expect("works"));
    let g = Box::new(NativeGuiDepends::new(*node_config.clone()));
    // let c = g.get_config();
    let ret = gui::initialize::prepare_start(node_config, res, g, party_data).await.expect("GUI to start");
    ret
}

// #[tokio::main]
// async fn main() {
async fn main_dbg(node_config: Box<NodeConfig>) {
    //
    // let (node_config, abort) = load_configs().await;
    // if abort {
    //     return
    // }
    //
    // // info!("Starting node main method");
    // counter!("redgold.node.main_started").increment(1);
    //
    // tracing::trace!("Starting network environment: {}", node_config.clone().network.to_std_string());


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
