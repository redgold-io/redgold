use crate::data::data_store::{DataStore, MnemonicEntry};
use crate::node_config::NodeConfig;
use crate::schema::structs::NetworkEnvironment;
use crate::{canary, gui, util};
use bitcoin_wallet::account::MasterKeyEntropy;
use bitcoin_wallet::mnemonic::Mnemonic;
use clap::{Args, Parser, Subcommand};
use crypto::digest::Digest;
#[allow(unused_imports)]
use futures::StreamExt;
use log::info;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use itertools::Itertools;
use tokio::runtime::Runtime;
use redgold_schema::structs::ErrorInfo;
use crate::core::seeds::SeedNode;
use crate::util::cli::{args, commands};
use crate::util::cli::args::{RgArgs, RgTopLevelSubcommand};
use crate::util::cli::commands::mnemonic_fingerprint;
use crate::util::ip_lookup;

// https://github.com/mehcode/config-rs/blob/master/examples/simple/src/main.rs

pub fn get_default_data_directory(network_type: NetworkEnvironment) -> PathBuf {
    let home_or_current = dirs::home_dir()
        .expect("Unable to find home directory for default data store path as path not specified explicitly")
        .clone();
    let redgold_dir = home_or_current.join(".rg");
    let net_dir = redgold_dir.clone().join(network_type.to_std_string());
    net_dir
}

pub fn load_node_config_initial(opts: RgArgs, mut node_config: NodeConfig) -> NodeConfig {

    let subcmd: Option<RgTopLevelSubcommand> = opts.subcmd;
    let mut is_canary = false;
    match subcmd {
        Some(c) => match c {
            RgTopLevelSubcommand::DebugCanary(_) => {
                is_canary = true;
            }
            _ => {}
        },
        _ => {}
    }

    node_config.network = match opts.network {
        None => {
            if util::local_debug_mode() {
                NetworkEnvironment::Debug
            } else {
                NetworkEnvironment::Local
            }
        }
        Some(mut n) => {
            let string2 = util::make_ascii_titlecase(&mut *n);
            NetworkEnvironment::from_str(&*string2).expect("error parsing network environment")
        }
    };
    node_config.port_offset = node_config.network.default_port_offset();

    if node_config.network == NetworkEnvironment::Local || node_config.network == NetworkEnvironment::Debug {
        node_config.disable_auto_update = true;
    }

    let ds_path_opt: Option<String> = opts.data_store_path;

    let data_store_file_path = match ds_path_opt {
        None => {
            let mut net_dir = get_default_data_directory(node_config.network);
            if is_canary {
                net_dir = net_dir.join("canary");
            }
            let dbg_id: Option<i32> = opts.debug_id ;
            if dbg_id.is_some() {
                net_dir = net_dir.join(format!("{}", dbg_id.unwrap()));
            }
            let ds_path = net_dir.as_path().clone();

            // let result = fs::try_exists(ds_path.clone()).unwrap_or(false);
            // if !result {
                info!(
                    "Ensuring make directory for datastore in: {:?}",
                    ds_path.clone().to_str()
                );
                fs::create_dir_all(ds_path).expect("Directory unable to be created.");
            // }
            ds_path
                .join("data_store.sqlite")
                .as_path()
                .to_str()
                .expect("Path format error")
                .to_string()
        }
        Some(p) => p,
    };

    let dbg_id: Option<i32> = opts.debug_id;
    if dbg_id.is_some() {
        let dbg_id = dbg_id.unwrap();
        let offset = ((dbg_id * 1000) as u16);
        node_config.port_offset = node_config.network.default_port_offset() + offset;
    }

    node_config.data_store_path = data_store_file_path;
    node_config
}

pub fn load_node_config(
    runtime: Arc<Runtime>,
    opts: RgArgs,
    mut node_config: NodeConfig,
) -> Result<NodeConfig, NodeConfig> {
    // let mut node_config = NodeConfig::default();
    let opts2 = opts.clone();

    let subcmd: Option<RgTopLevelSubcommand> = opts.subcmd;
    let mut is_canary = false;
    match subcmd.clone() {
        None => {}
        Some(c) => match c {
            RgTopLevelSubcommand::GenerateWords(_) => {}
            RgTopLevelSubcommand::DebugCanary(_) => {
                is_canary = true;
            }
            RgTopLevelSubcommand::GUI(_) => {

            }
            RgTopLevelSubcommand::Deploy(_) => {
                // TODO: Get this from a master config database.

                // actually let's just not introduce this yet as it should really be something
                // based on commit hashes etc. for proper CI so we have a built commit that
                // matches properly.
                // infra::
            }
            _ => {}
        },
    }

    info!(
        "Starting node with data store path: {}",
        node_config.data_store_path
    );

    let store = runtime.block_on(node_config.data_store());
    // runtime.block_on(
    // store
    //     .create_all_err_info()
    //     // )
    //     .expect("Unable to create initial tables");
    //
    // runtime
    //     .block_on(store.create_mnemonic())
    //     .expect("Create mnemonic");
    //
    // let mnemonic_store = runtime
    //     .block_on(store.query_all_mnemonic())
    //     .expect("query success");
    let mnemonic_from_store: Option<String> = None; //mnemonic_store.get(0).map(|x| x.words.clone());
    // TODO: Use this
    let _peer_id_from_store: Option<String> = None; // mnemonic_store.get(0).map(|x| x.peer_id.clone());

    let opt_mnemonic: Option<String> = opts.words;
    let path_mnemonic: Option<String> = opts
        .mnemonic_path
        .map(fs::read_to_string)
        .map(|x| x.expect("Something went wrong reading the mnemonic_path file"));
    let mut default_data_dir = get_default_data_directory(node_config.network);
    let dbg_id: Option<i32> = opts.debug_id;
    if dbg_id.is_some() {
        let dbg_id = dbg_id.unwrap();
        default_data_dir = default_data_dir.join(format!("{}", dbg_id));
        let offset = ((dbg_id * 1000) as u16);
        node_config.port_offset = node_config.network.default_port_offset() + offset;
    }

    let default_path_mnemonic_path = &default_data_dir.join("mnemonic");
    let default_path_peer_id_path = &default_data_dir.join("peer_id");
    let default_path_mnemonic: Option<String> = {
        if default_path_mnemonic_path.clone().exists() {
            Some(fs::read_to_string(default_path_mnemonic_path.clone()).expect("Something went wrong reading the default mnemonic_path file"))
        } else {
            None
        }
    };
    let default_path_peer_id_hex: Option<String> = {
        if default_path_peer_id_path.clone().exists() {
            Some(fs::read_to_string(default_path_peer_id_path.clone()).expect("Something went wrong reading the default mnemonic_path file"))
        } else {
            None
        }
    };

    // let ds_mnemonic = ;
    let mnemonic_load_order = vec![opt_mnemonic, path_mnemonic, default_path_mnemonic.clone(), mnemonic_from_store.clone()];
    let chosen = mnemonic_load_order
        .iter()
        .filter(|m| m.is_some())
        .map(|m| m.clone())
        .collect::<Vec<Option<String>>>()
        .get(0)
        .map(|x| x.clone())
        .flatten();

    let mnemonic = match chosen {
        None => {
            // Move this to separate operation to fill in missing configs after the fact?
            log::info!(
                "Unable to load mnemonic for wallet / node keys, attempting to generate new one"
            );
            log::info!(
                "Generating with MasterKeyEntropy::Paranoid -- \
        process may halt if insufficient entropy on system"
            );
            let mnem = Mnemonic::new_random(MasterKeyEntropy::Paranoid)
                .expect("New mnemonic generation failure");
            log::info!("Successfully generated new mnemonic");
            // store to ds.
            mnem
        }
        Some(c) => {
            let mnemonic1 = Mnemonic::from_str(&c).expect("Mnemonic is incorrectly formatted");
            info!("Loaded existing mnemonic with fingerprint: {}", mnemonic_fingerprint(mnemonic1.clone()));
            mnemonic1
        },
    };

    // TODO: Make this the actual class, way easier now
    node_config.mnemonic_words = mnemonic.to_string();

    let peer_id_opt: Option<String> = opts.peer_id.clone();
    let peer_id_path_opt: Option<String> = opts.peer_id_path.clone();
    let peer_id_from_opt: Option<Vec<u8>> = (peer_id_opt)
        .or_else(|| {
            peer_id_path_opt.map(|p| {
                fs::read_to_string(p).expect("Something went wrong reading the peer_id_path file")
            })
        })
        .or_else(|| {
            default_path_peer_id_hex
        })
        .map(|p| hex::decode(p).expect("Hex decode failure on peer id"));

    node_config.self_peer_id = match peer_id_from_opt {
        None => {
            info!("No peer_id found, attempting to generate a single key peer_id from mnemonic");
            let string = mnemonic.to_string();
            crate::node_config::debug_peer_id_from_key(&*string).to_vec()
        }
        Some(r) => r,
    };

    if mnemonic_from_store.is_none() || default_path_mnemonic.is_none() {
        // let insert = store.insert_mnemonic(MnemonicEntry {
        //     words: node_config.mnemonic_words.clone(),
        //     time: util::current_time_millis() as i64,
        //     peer_id: node_config.self_peer_id.clone(),
        // });
        std::fs::write(default_path_mnemonic_path.clone(), &node_config.mnemonic_words.clone()).expect("Unable to write mnemonic to file");
        std::fs::write(default_path_peer_id_path.clone(), hex::encode(&node_config.self_peer_id.clone())).expect("Unable to write peer id to file");
        // let err = runtime.block_on(insert);
        // DataStore::map_err_sqlx(err);
    }

    let path_exec = std::env::current_exe().expect("Can't find the current exe");

    let buf1 = path_exec.clone();
    let path_str = buf1.to_str().expect("Path exec format failure");
    info!("Path of current executable: {:?}", path_str);
    let exec_name = path_exec.file_name().expect("filename access failure").to_str()
        .expect("Filename missing").to_string();
    info!("Filename of current executable: {:?}", exec_name.clone());
    let mut file = fs::File::open(path_exec.clone())
        .expect("Can't open self file executable for calculating md5");

    let mut buf: Vec<u8> = vec![];
    file.read(&mut *buf).expect("works");
    let mut md5f = crypto::md5::Md5::new();
    md5f.input(&*buf);

    info!(
        "Md5 of currently running executable with read byte {}",
        md5f.result_str()
    );

    // breaks locally
    // let mut strr = "".to_string();
    // file.read_to_string(&mut strr).expect("read");
    // let mut md5 = crypto::md5::Md5::new();
    // md5.input_str(&*strr);
    //
    // let res = md5.result_str();
    //
    // info!("Md5 of currently running executable with read str {}", res);

    use std::process::Command;

    // let mut echo_hello = Command::new("md5sum");
    // echo_hello.arg(path_str.clone());
    // let hello_1 = echo_hello.output().expect("Ouput from command failure");
    // let string1 = String::from_utf8(hello_1.stdout).expect("String decode failure");
    // let md5stdout: String = string1
    //     .split_whitespace()
    //     .next()
    //     .expect("first output")
    //     .clone()
    //     .to_string();

    // info!("Md5sum stdout from shell script: {}", md5stdout);

    let shasum = calc_sha_sum(path_str.clone().to_string());

    // let exec = dirs::executable_dir().expect("Can't find self-executable dir");
    node_config.executable_checksum = Some(shasum.clone());
    info!("Sha256 checksum from shell script: {}", shasum);
    //
    // if is_canary {
    //     canary::run(node_config.clone());
    //     return Err(node_config.clone());
    // }

    // let external_ip = ip_lookup::get_self_ip().await?;
    // node_config.
    let mut abort = true;
    match subcmd {
        None => {abort = false}
        Some(c) => match c {
            RgTopLevelSubcommand::GenerateWords(m) => {
                commands::generate_mnemonic(&m);
            }
            // RgTopLevelSubcommand::AddServer(a) => {
            //     commands::add_server(&a, &node_config);
            // }
            _ => {
                abort = false;
            }
        }
    }
    if abort {
        return Err(node_config);
    }

    // Only enable on main if CLI flag
    if node_config.network == NetworkEnvironment::Main {
        node_config.faucet_enabled = false;
        //match subcmd
    }

    // TODO: Check ports open in separate thing

    // TODO: Also set from HOSTNAME maybe? With nslookup for confirmation of IP?
    if !node_config.is_local_debug() && node_config.external_ip == "127.0.0.1".to_string() {
        let ip = runtime.block_on(ip_lookup::get_self_ip()).expect("Ip lookup failed");
        info!("Assigning external IP from ip lookup: {}", ip);
        node_config.external_ip = ip;
    }

    // let mut at = ArgTranslate::new(runtime.clone(), &opts2, node_config);

    // at.parse_seed();

    Ok(node_config)

}

pub struct ArgTranslate {
    runtime: Arc<Runtime>,
    pub opts: RgArgs,
    pub node_config: NodeConfig,
    pub args: Vec<String>,
}

impl ArgTranslate {

    pub fn new(runtime: Arc<Runtime>, opts: &RgArgs, node_config: NodeConfig) -> Self {
        let args = std::env::args().collect_vec();
        ArgTranslate {
            runtime,
            opts: opts.clone(),
            node_config,
            args,
        }
    }

    pub fn is_gui(&self) -> bool {
        if let Some(sc) = &self.opts.subcmd {
            match sc {
                RgTopLevelSubcommand::GUI(_) => {
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    // pub fn parse_seed(&mut self) {
    //     if let Some(a) = &self.opts.seed_address {
    //         let default_port = self.node_config.network.default_port_offset();
    //         let port = self.opts.seed_port_offset.map(|p| p as u16).unwrap_or(default_port);
    //         self.node_config.seeds.push(SeedNode {
    //             peer_id: vec![],
    //             trust: 1.0,
    //             public_key: None,
    //             external_address: a.clone(),
    //             port
    //         });
    //     }
    // }
}



fn calc_sha_sum(path: String) -> String {
    util::cmd::run_cmd("shasum", vec!["-a", "256", &*path])
        .0
        .split_whitespace()
        .next()
        .expect("first output")
        .clone()
        .to_string()
}

// #[tokio::test]
// async fn debug_open_database() {
//     util::init_logger().ok(); //expect("log");
//     let net_dir = get_default_data_directory(NetworkEnvironment::Local);
//     let ds_path = net_dir.as_path().clone();
//     info!(
//         "Attempting to make directory for datastore in: {:?}",
//         ds_path.clone().to_str()
//     );
//     fs::create_dir_all(ds_path).expect("Directory unable to be created.");
//     let path = ds_path
//         .join("data_store.sqlite")
//         .as_path()
//         .to_str()
//         .expect("Path format error")
//         .to_string();
//
//     let mut node_config = NodeConfig::default();
//     node_config.data_store_path = path.clone();
//     info!("Using path: {}", path);
//
//     let store = node_config.data_store().await;
//     store
//         .create_all_err_info()
//         // .await
//         .expect("Unable to create initial tables");
//
//     store.create_mnemonic().await.expect("Create mnemonic");
// }

#[test]
fn test_shasum() {
    println!("{}", calc_sha_sum("Cargo.toml".to_string()));
}

#[test]
fn load_ds_path() {
    let mut config = NodeConfig::default();
    let res = load_node_config_initial(args::empty_args(), config);
    println!("{}", res.data_store_folder());
}

// TODO: Settings from config if necessary
/*    let mut settings = config::Config::default();
    let mut settings2 = settings.clone();
    settings
        // Add in `./Settings.toml`
        .merge(config::File::with_name("Settings"))
        .unwrap_or(&mut settings2)
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        .merge(config::Environment::with_prefix("REDGOLD"))
        .unwrap();
*/
// Pre logger commands
pub fn immediate_commands(opts: &RgArgs, config: &NodeConfig, simple_runtime: Arc<Runtime>) -> bool {
    let mut abort = false;
    let res: Result<(), ErrorInfo> = match &opts.subcmd {
        None => {Ok(())}
        Some(c) => {
            abort = true;
            match c {
                RgTopLevelSubcommand::GenerateWords(m) => {
                    commands::generate_mnemonic(&m);
                    Ok(())
                },
                RgTopLevelSubcommand::Address(a) => {
                    commands::generate_address(a.clone(), &config);
                    Ok(())
                }
                RgTopLevelSubcommand::Send(a) => {
                    let res = simple_runtime.block_on(commands::send(&a, &config));
                    res
                }
                RgTopLevelSubcommand::Query(a) => {
                    commands::query(&a, &config);
                    Ok(())
                }
                RgTopLevelSubcommand::Faucet(a) => {
                    simple_runtime.block_on(commands::faucet(&a, &config))
                }
                RgTopLevelSubcommand::AddServer(a) => {
                    simple_runtime.block_on(commands::add_server(a, &config))
                }

                _ => {
                    abort = false;
                    Ok(())
                }
            }
        }
    };
    if res.is_err() {
        println!("{}", serde_json::to_string(&res.err().unwrap()).expect("json"));
        abort = true;
    }
    abort
}