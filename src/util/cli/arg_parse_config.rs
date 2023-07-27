use crate::data::data_store::{DataStore, MnemonicEntry};
use crate::node_config::NodeConfig;
use crate::schema::structs::NetworkEnvironment;
use crate::{e2e, gui, util};
use bitcoin_wallet::account::MasterKeyEntropy;
use bitcoin_wallet::mnemonic::Mnemonic;
use clap::{Args, Parser, Subcommand};
use crypto::digest::Digest;
#[allow(unused_imports)]
use futures::StreamExt;
use log::{error, info};
use std::fs;
use std::io::Read;
use std::net::{AddrParseError, IpAddr, SocketAddr};
use std::path::PathBuf;
use std::process::abort;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use bitcoin::bech32::ToBase32;
use crypto::sha2::Sha256;
use itertools::Itertools;
use tokio::runtime::Runtime;
use redgold_schema::{ErrorInfoContext, from_hex, SafeOption};
use redgold_schema::seeds::get_seeds;
use redgold_schema::servers::Server;
use redgold_schema::structs::{ErrorInfo, Hash, PeerId, Seed, TrustData};
use crate::core::seeds::SeedNode;
use crate::util::cli::{args, commands};
use crate::util::cli::args::{GUI, NodeCli, RgArgs, RgTopLevelSubcommand};
use crate::util::cli::commands::mnemonic_fingerprint;
use crate::util::cli::data_folder::DataFolder;
use crate::util::{init_logger, init_logger_main, ip_lookup, metrics_registry, not_local_debug_mode, sha256_vec};
use crate::util::trace_setup::init_tracing;

// https://github.com/mehcode/config-rs/blob/master/examples/simple/src/main.rs

pub fn get_default_data_top_folder() -> PathBuf {
    let home_or_current = dirs::home_dir()
        .expect("Unable to find home directory for default data store path as path not specified explicitly")
        .clone();
    let redgold_dir = home_or_current.join(".rg");
    redgold_dir
}

use redgold_schema::EasyJson;


pub struct ArgTranslate {
    // runtime: Arc<Runtime>,
    pub opts: RgArgs,
    pub node_config: NodeConfig,
    pub args: Vec<String>,
    pub abort: bool,
}

impl ArgTranslate {

    pub fn new(
        // runtime: Arc<Runtime>,
        opts: &RgArgs, node_config: &NodeConfig) -> Self {
        let args = std::env::args().collect_vec();
        ArgTranslate {
            // runtime,
            opts: opts.clone(),
            node_config: node_config.clone(),
            args,
            abort: false
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

    pub fn secure_data_path_string() -> Option<String> {
        std::env::var("REDGOLD_SECURE_DATA_PATH").ok()
    }

    pub fn secure_data_path_buf() -> Option<PathBuf> {
        std::env::var("REDGOLD_SECURE_DATA_PATH").ok().map(|a| PathBuf::from(a))
    }

    pub fn secure_data_or_cwd() -> PathBuf {
        Self::secure_data_path_string().map(|s|
            std::path::Path::new(&s).to_path_buf()
        ).unwrap_or(std::env::current_dir().ok().expect("Can't get current dir"))
    }

    pub fn load_internal_servers(&mut self) -> Result<(), ErrorInfo> {
        let data_folder = Self::secure_data_or_cwd();
        let rg = data_folder.join(".rg");
        let all = rg.join(NetworkEnvironment::All.to_std_string());
        let servers = all.join("servers");
        if servers.is_file() {
            let contents = fs::read_to_string(servers)
                .error_info("Failed to read servers file")?;
            let servers = Server::parse(contents)?;
            self.node_config.servers = servers;
        }
        Ok(())
    }

    pub fn read_servers_file(servers: PathBuf) -> Result<Vec<Server>, ErrorInfo> {
        let result = if servers.is_file() {
            let contents = fs::read_to_string(servers)
                .error_info("Failed to read servers file")?;
            let servers = Server::parse(contents)?;
            servers
        } else {
            vec![]
        };
        Ok(result)
    }

    pub async fn translate_args(&mut self) -> Result<(), ErrorInfo> {
        self.set_gui_on_empty();
        self.check_load_logger()?;
        self.determine_network()?;
        self.ports();
        metrics_registry::register_metrics(self.node_config.port_offset);
        self.data_folder()?;
        self.load_mnemonic()?;
        self.load_peer_id()?;
        self.load_internal_servers()?;
        self.calculate_executable_checksum_hash();
        // No logger for CLI commands to allow direct output read.
        self.abort = immediate_commands(&self.opts, &self.node_config).await;
        if self.abort {
            return Ok(());
        }

        self.guard_faucet();
        self.lookup_ip().await;

        self.e2e_enable();
        self.set_public_key();
        self.configure_seeds();
        self.set_discovery_interval();


        self.apply_node_opts();
        self.genesis();

        tracing::info!("Starting node with data store path: {}", self.node_config.data_store_path());
        tracing::info!("Parsed args successfully with args: {:?}", self.args);
        tracing::info!("RgArgs options parsed: {:?}", self.opts);

        Ok(())
    }

    fn set_discovery_interval(&mut self) {
        if !self.node_config.is_local_debug() {
            self.node_config.discovery_interval = Duration::from_secs(60)
        }
    }

    fn guard_faucet(&mut self) {
        // Only enable on main if CLI flag with additional precautions
        if self.node_config.network == NetworkEnvironment::Main {
            self.node_config.faucet_enabled = false;
        }
    }

    async fn lookup_ip(&mut self) {

        std::env::var("REDGOLD_EXTERNAL_IP").ok().map(|a| {
            // TODO: First determine if this is an nslookup requirement
            let parsed = IpAddr::from_str(&a);
            match parsed {
                Ok(_) => {
                    self.node_config.external_ip = a;
                }
                Err(_) => {
                    let lookup = dns_lookup::lookup_host(&a);
                    match lookup {
                        Ok(addr) => {
                            if addr.len() > 0 {
                                self.node_config.external_ip = addr[0].to_string();
                            }
                        }
                        Err(_) => {
                            error!("Invalid REDGOLD_EXTERNAL_IP environment variable: {}", a);
                        }
                    }
                }
            }
            // self.node_config.external_ip = a;
        });
        // TODO: We can use the lb or another node to check if port is reciprocal open
        // TODO: Check ports open in separate thing
        // TODO: Also set from HOSTNAME maybe? With nslookup for confirmation of IP?
        if !self.node_config.is_local_debug() && self.node_config.external_ip == "127.0.0.1".to_string() {
            let ip =
                // runtime.block_on(
                ip_lookup::get_self_ip()
                    .await
                    .expect("Ip lookup failed");
            info!("Assigning external IP from ip lookup: {}", ip);
            self.node_config.external_ip = ip;
        }
    }

    fn calculate_executable_checksum_hash(&mut self) {

        let path_exec = std::env::current_exe().expect("Can't find the current exe");

        let buf1 = path_exec.clone();
        let path_str = buf1.to_str().expect("Path exec format failure");
        info!("Path of current executable: {:?}", path_str);
        let exec_name = path_exec.file_name().expect("filename access failure").to_str()
            .expect("Filename missing").to_string();
        info!("Filename of current executable: {:?}", exec_name.clone());
        // This is somewhat slow for loading the GUI
        // let self_exe_bytes = fs::read(path_exec.clone()).expect("Read bytes of current exe");
        // let mut md5f = crypto::md5::Md5::new();
        // md5f.input(&*self_exe_bytes);
        //
        // info!("Md5 of currently running executable with read byte {}", md5f.result_str());
        // let sha256 = sha256_vec(&self_exe_bytes);
        // info!("Sha256 of currently running executable with read byte {}", hex::encode(sha256.to_vec()));

        // let sha3_256 = Hash::calc_bytes(self_exe_bytes);
        // info!("Sha3-256 of current exe {}", sha3_256.hex());

        use std::process::Command;

        let shasum = calc_sha_sum(path_str.clone().to_string());

        self.node_config.executable_checksum = Some(shasum.clone());
        info!("Sha256 checksum from shell script: {}", shasum);
    }

    fn load_mnemonic(&mut self) -> Result<(), ErrorInfo> {

        // Remove any defaults; we want to be explicit
        self.node_config.mnemonic_words = "".to_string();

        // First load from environment
        if let Some(words) = std::env::var("REDGOLD_WORDS").ok() {
            self.node_config.mnemonic_words = words;
        };

        // Then override with optional found file
        // TODO: Should we just change this to an ALL folder?
        let mnemonic_disk_path = self.node_config.env_data_folder().mnemonic_path();
        if let Some(words) = fs::read_to_string(mnemonic_disk_path.clone()).ok() {
            self.node_config.mnemonic_words = words;
        };

        // Then override with command line
        if let Some(words) = &self.opts.words {
            self.node_config.mnemonic_words = words.clone();
        }

        // Then override with a file from the command line (more secure than passing directly)
        if let Some(words) = &self.opts
            .mnemonic_path
            .clone()
            .map(fs::read_to_string)
            .map(|x| x.expect("Something went wrong reading the mnemonic_path file")) {
            self.node_config.mnemonic_words = words.clone();
        };


        // If empty, generate a new mnemonic;
        if self.node_config.mnemonic_words.is_empty() {
            tracing::info!("Unable to load mnemonic for wallet / node keys, attempting to generate new one");
            tracing::info!("Generating with entropy for 24 words, process may halt if insufficient entropy on system");
            let mnem = Mnemonic::new_random(MasterKeyEntropy::Double).expect("New mnemonic generation failure");
            tracing::info!("Successfully generated new mnemonic");
            self.node_config.mnemonic_words = mnem.to_string();
        };

        // Validate that this is loadable
        let mnemonic = Mnemonic::from_str(&self.node_config.mnemonic_words)
            .error_info("Failed to parse mnemonic")?;

        // Re-assign as words to avoid coupling class to node config.
        self.node_config.mnemonic_words = mnemonic.to_string();


        // Attempt to write mnemonic to disk for persistence
            // let insert = store.insert_mnemonic(MnemonicEntry {
            //     words: node_config.mnemonic_words.clone(),
            //     time: util::current_time_millis() as i64,
            //     peer_id: node_config.self_peer_id.clone(),
        //     // });
        //     std::fs::create_dir_all(mnemonic_disk_path.clone()).expect("Unable to create data dir");
        fs::write(mnemonic_disk_path.clone(), self.node_config.mnemonic_words.clone()).expect("Unable to write mnemonic to file");


        Ok(())
    }

    // TODO: Load merkle tree of this
    fn load_peer_id(&mut self) -> Result<(), ErrorInfo> {
        // // TODO: Use this
        // let _peer_id_from_store: Option<String> = None; // mnemonic_store.get(0).map(|x| x.peer_id.clone());

        // TODO: From environment variable too?
        // TODO: write merkle tree to disk

        if let Some(path) = &self.opts.peer_id_path {
            let p = fs::read_to_string(path)
                .error_info("Failed to read peer_id_path file")?;
            self.node_config.self_peer_id = from_hex(p)?;
        }

        // TODO: This will have to change to read the whole merkle tree really, lets just remove this maybe?
        if let Some(p) = &self.opts.peer_id {
            self.node_config.self_peer_id = from_hex(p.clone())?;
        }

        if let Some(p) = fs::read_to_string(self.node_config.env_data_folder().peer_id_path()).ok() {
            self.node_config.self_peer_id = from_hex(p.clone())?;
        }

        if self.node_config.self_peer_id.is_empty() {
            tracing::info!("No peer_id found, attempting to generate a single key peer_id from existing mnemonic");
            let string = self.node_config.mnemonic_words.clone();
            // TODO: we need to persist the merkle tree here as json or something
            let tree = crate::node_config::peer_id_from_single_mnemonic(string)?;
            self.node_config.self_peer_id = tree.root.vec();
        }

        Ok(())

    }

    fn data_folder(&mut self) -> Result<(), ErrorInfo> {

        let mut data_folder_path =  self.opts.data_folder.clone()
            .map(|p| PathBuf::from(p))
            .unwrap_or(get_default_data_top_folder());

        // Testing only modification, could potentially do this in a separate function to
        // unify this with other debug mods.
        if let Some(id) = self.opts.debug_id {
            data_folder_path = data_folder_path.join("local_test");
            data_folder_path = data_folder_path.join(format!("id_{}", id));
        }

        self.node_config.data_folder = DataFolder { path: data_folder_path };
        self.node_config.data_folder.ensure_exists();
        self.node_config.env_data_folder().ensure_exists();

        Ok(())
    }

    fn ports(&mut self) {
        self.node_config.port_offset = self.node_config.network.default_port_offset();

        // Unify with other debug id stuff?
        if let Some(dbg_id) = self.opts.debug_id {
            self.node_config.port_offset = Self::debug_id_port_offset(
                self.node_config.network.default_port_offset(),
                dbg_id
            );
        }
    }

    fn debug_id_port_offset(offset: u16, debug_id: i32) -> u16 {
        offset + ((debug_id * 1000) as u16)
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
    fn check_load_logger(&mut self) -> Result<(), ErrorInfo> {
        let log_level = &self.opts.log_level
            .clone()
            .and(std::env::var("REDGOLD_LOG_LEVEL").ok())
            .unwrap_or("DEBUG".to_string());
        let mut enable_logger = false;

        if let Some(sc) = &self.opts.subcmd {
            enable_logger = match sc {
                RgTopLevelSubcommand::GUI(_) => { true }
                RgTopLevelSubcommand::Node(_) => { true }
                RgTopLevelSubcommand::TestTransaction(_) => {true}
                _ => { false }
            }
        }
        if enable_logger {
            init_logger_main(log_level.clone());
        }
        self.node_config.enable_logging = enable_logger;
        self.node_config.log_level = log_level.clone();


        Ok(())
    }
    fn determine_network(&mut self) -> Result<(), ErrorInfo> {
        if let Some(n) = std::env::var("REDGOLD_NETWORK").ok() {
            NetworkEnvironment::parse_safe(n)?;
        }
        self.node_config.network = match &self.opts.network {
            None => {
                if util::local_debug_mode() {
                    NetworkEnvironment::Debug
                } else {
                    NetworkEnvironment::Local
                }
            }
            Some(n) => {
                NetworkEnvironment::parse_safe(n.clone())?
            }
        };

        if self.node_config.network == NetworkEnvironment::Local || self.node_config.network == NetworkEnvironment::Debug {
            self.node_config.disable_auto_update = true;
            self.node_config.load_balancer_url = "127.0.0.1".to_string();
        }
        Ok(())
    }

    fn e2e_enable(&mut self) {

        if self.opts.disable_e2e {
            self.node_config.e2e_enabled = false;
        }
        // std::env::var("REDGOLD_ENABLE_E2E").ok().map(|b| {
        //     self.node_config.e2e_enable = true;
        // }
        // self.opts.enable_e2e.map(|_| {
        //     self.node_config.e2e_enable = true;
        // });
    }
    fn configure_seeds(&mut self) {

        let seeds = get_seeds();
        for seed in seeds {
            let env_match = seed.environments.contains(&(self.node_config.network as i32));
            let all_env = !self.node_config.is_local_debug() &&
                seed.environments.contains(&(NetworkEnvironment::All as i32));
            if env_match || all_env {
                self.node_config.seeds.push(seed);
            }
        }

        if let Some(a) = &self.opts.seed_address {
            let default_port = self.node_config.network.default_port_offset();
            let port = self.opts.seed_port_offset.map(|p| p as u16).unwrap_or(default_port);
            // TODO: replace this with the other seed class.
            self.node_config.seeds.push(Seed {
                external_address: a.clone(),
                environments: vec![self.node_config.network as i32],
                port_offset: Some(port as u32),
                trust: vec![TrustData::from_label(1.0)],
                peer_id: Some(self.node_config.peer_id()),
                public_key: Some(self.node_config.public_key()),
            });
        }
    }
    fn apply_node_opts(&mut self) {
        match &self.opts.subcmd {
            Some(RgTopLevelSubcommand::Node(node_cli)) => {
                if let Some(i) = &node_cli.live_e2e_interval {
                    self.node_config.live_e2e_interval = Duration::from_secs(i.clone());
                }
            }
            _ => {}
        }
    }
    fn genesis(&mut self) {
        if let Some(o) = std::env::var("REDGOLD_GENESIS").ok() {
            if let Ok(b) = o.parse::<bool>() {
                self.node_config.genesis = b;
            }
        }
        if self.opts.genesis {
            self.node_config.genesis = true;
        }
        if self.node_config.genesis {
            self.node_config.seeds.push(self.node_config.self_seed())
        }
        if self.node_config.genesis {
            info!("Starting node as genesis node");
        }
    }
    fn set_gui_on_empty(&mut self) {
        // println!("args: {:?}", self.args.clone());
        if self.args.len() == 1 {
            self.opts.subcmd = Some(RgTopLevelSubcommand::GUI(GUI{}));
        }
    }
    fn set_public_key(&mut self) {
        let pk = self.node_config.public_key();
        self.node_config.public_key = pk.clone();
        info!("Starting node with public key: {}", pk.json_or());
    }
}


/**
This function uses an external program for calculating checksum.
Tried doing this locally, but for some reason it seemed to have a different output than the shell script.
There's internal libraries for getting the current exe path and calculating checksum, but they
seem to produce a different result than the shell script.
*/
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
    let config = NodeConfig::default();
    // let res = load_node_config_initial(args::empty_args(), config);
    // println!("{}", res.data_store_path());
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
pub async fn immediate_commands(opts: &RgArgs, config: &NodeConfig
                          // , simple_runtime: Arc<Runtime>
) -> bool {
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
                    commands::generate_address(a.clone(), &config).map(|_| ())
                }
                RgTopLevelSubcommand::Send(a) => {
                    commands::send(&a, &config).await
                }
                RgTopLevelSubcommand::Query(a) => {
                    commands::query(&a, &config).await
                }
                RgTopLevelSubcommand::Faucet(a) => {
                    commands::faucet(&a, &config).await
                }
                RgTopLevelSubcommand::AddServer(a) => {
                    commands::add_server(a, &config).await
                }
                RgTopLevelSubcommand::Balance(a) => {
                    commands::balance_lookup(a, &config).await
                }
                RgTopLevelSubcommand::TestTransaction(test_transaction_cli) => {
                    commands::test_transaction(&test_transaction_cli, &config).await
                }
                RgTopLevelSubcommand::Deploy(d) => {
                    commands::deploy(d, &config).await
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