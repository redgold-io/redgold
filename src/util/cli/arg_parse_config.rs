use std::fs;
use std::io::Read;
use std::net::{AddrParseError, IpAddr, SocketAddr};
use std::path::PathBuf;
use std::process::{abort, exit};
use std::slice::Iter;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use bdk::bitcoin::bech32::ToBase32;
use clap::{Args, Parser, Subcommand};
use crypto::digest::Digest;
use crypto::sha2::Sha256;
#[allow(unused_imports)]
use futures::StreamExt;
use itertools::Itertools;
use metrics::{gauge, Label};
use tokio::runtime::Runtime;
use tracing::{error, info, trace};
use redgold_common_no_wasm::data_folder_read_ext::EnvFolderReadExt;
use redgold_data::data_store::DataStore;
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_schema::{error_info, from_hex, ErrorInfoContext, RgResult, SafeOption};
use redgold_schema::constants::default_node_internal_derivation_path;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::easy_json::EasyJsonDeser;
use redgold_schema::seeds::{get_seeds_by_env, get_seeds_by_env_time};
use redgold_schema::servers::ServerOldFormat;
use redgold_schema::structs::{ErrorInfo, Hash, PeerId, Seed, Transaction, TrustData};

use crate::{e2e, gui, util};
use crate::api::RgHttpClient;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::conf::rg_args::{NodeCli, RgArgs, RgTopLevelSubcommand, TestCaptureCli, GUI};
use redgold_schema::config_data::ConfigData;
// use crate::gui::image_capture::debug_capture;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::proto_serde::ProtoSerde;
use crate::observability::metrics_registry;
use crate::schema::structs::NetworkEnvironment;
use crate::util::{init_logger, init_logger_main, ip_lookup, not_local_debug_mode, sha256_vec};
use crate::util::cli::{args, commands, immediate_commands};
use redgold_schema::data_folder::DataFolder;
use crate::node_config::WordsPassNodeConfig;
// https://github.com/mehcode/config-rs/blob/master/examples/simple/src/main.rs

pub fn get_default_data_top_folder() -> PathBuf {
    let home_or_current = dirs::home_dir()
        .expect("Unable to find home directory for default data store path as path not specified explicitly")
        .clone();
    let redgold_dir = home_or_current.join(".rg");
    redgold_dir
}

pub struct ArgTranslate {
    pub node_config: Box<NodeConfig>,
    pub determined_subcommand: Option<RgTopLevelSubcommand>,
    pub abort: bool,
    pub opts: RgArgs,
}

impl ArgTranslate {

    pub fn new(
        node_config: Box<NodeConfig>,
        opts: RgArgs
    ) -> Self {
        
        ArgTranslate {
            node_config,
            determined_subcommand: None,
            abort: false,
            opts
        }
    }
    
    pub fn opts(&self) -> &RgArgs {
        &self.opts
    }

    pub fn is_gui(&self) -> bool {
        if let Some(sc) = self.get_subcommand() {
            match sc {
                RgTopLevelSubcommand::GUI(_) => {
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    fn get_subcommand(&self) -> Option<&RgTopLevelSubcommand> {
        self.determined_subcommand.as_ref().or(self.opts().subcmd.as_ref())
    }

    pub fn is_node(&self) -> bool {
        if let Some(sc) = self.get_subcommand() {
            match sc {
                RgTopLevelSubcommand::Node(_) => {
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    pub fn secure_data_path_string() -> Option<String> {
        std::env::var("REDGOLD_SECURE_PATH").ok()
    }

    pub fn secure_data_path_buf() -> Option<PathBuf> {
        std::env::var("REDGOLD_SECURE_PATH").ok().map(|a| PathBuf::from(a))
    }

    pub fn secure_data_or_cwd() -> PathBuf {
        Self::secure_data_path_string().map(|s|
            std::path::Path::new(&s).to_path_buf()
        ).unwrap_or(std::env::current_dir().ok().expect("Can't get current dir"))
    }

    pub fn load_internal_servers(&mut self) -> Result<(), ErrorInfo> {
        // TODO: From better data folder options
        let data_folder = Self::secure_data_or_cwd();
        let rg = data_folder.join(".rg");
        let df = DataFolder::from_path(rg);
        if let Some(servers) = df.all().servers().ok() {
            self.node_config.servers = servers;
        }
        Ok(())
    }

    pub fn read_servers_file(servers: PathBuf) -> Result<Vec<ServerOldFormat>, ErrorInfo> {
        let result = if servers.is_file() {
            let contents = fs::read_to_string(servers)
                .error_info("Failed to read servers file")?;
            let servers = ServerOldFormat::parse(contents)?;
            servers
        } else {
            vec![]
        };
        Ok(result)
    }

    pub async fn translate_args(mut self) -> Result<Box<NodeConfig>, ErrorInfo> {

        let skip_slow_stuff = self.determined_subcommand.clone().map(|x|
            match x {
                RgTopLevelSubcommand::GUI(_) => false,
                RgTopLevelSubcommand::Node(_) => false,
                _ => true
            }
        ).unwrap_or(false);

        self.immediate_debug();
        self.set_gui_on_empty();
        self.check_load_logger()?;
        self.determine_network()?;
        self.ports();
        if !self.node_config.disable_metrics {
            metrics_registry::register_metrics(self.node_config.port_offset);
        }
        self.data_folder()?;
        self.secure_data_folder();
        self.load_mnemonic()?;
        self.load_peer_id()?;
        self.load_peer_tx()?;
        self.set_public_key();
        self.load_internal_servers()?;
        if !skip_slow_stuff {
            self.calculate_executable_checksum_hash();
        }
        // info!("E2e enabled");
        if !skip_slow_stuff {
            self.configure_seeds().await;
        }
        self.set_discovery_interval();
        self.genesis();

        if self.is_gui() {
            self.node_config.is_gui = true;
            return Ok(self.node_config);
        }

        // let subcmd = self.opts.subcmd.as_ref().map(|x| Box::new(x.clone()));
        // self.node_config.top_level_subcommand = subcmd.clone();

        if !skip_slow_stuff {
            self.lookup_ip().await;
        }
        // Unnecessary for CLI commands, hence after immediate commands
        // self.lookup_ip().await;

        tracing::debug!("Starting node with data store path: {}", self.node_config.data_store_path());
        tracing::info!("Parsed args successfully with args: {:?}", self.args());
        tracing::info!("RgArgs options parsed: {:?}", self.opts().clear_sensitive());
        // info!("Development mode: {}", self.opts().development_mode);

        Ok(self.node_config)
    }

    fn set_discovery_interval(&mut self) {
        if !self.node_config.is_local_debug() {
            self.node_config.discovery_interval = Duration::from_secs(300)
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
        if !self.node_config.is_local_debug() &&
            self.node_config.external_ip == "127.0.0.1".to_string() &&
            !self.is_gui() {
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
        trace!("Path of current executable: {:?}", path_str);
        let exec_name = path_exec.file_name().expect("filename access failure").to_str()
            .expect("Filename missing").to_string();
        trace!("Filename of current executable: {:?}", exec_name.clone());
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

        let shasum = calc_sha_sum(path_str.to_string()).log_error().ok();

        self.node_config.executable_checksum = shasum.clone();
        trace!("Executable checksum Sha256 from shell script: {:?}", shasum);
        let or = shasum.unwrap_or("na".to_string());
        let last_8 = or.chars().take(8).collect::<String>();
        gauge!("redgold.node.executable_checksum", &[("executable_checksum".to_string(), or)]).set(1.0);
        let id = self.node_config.gauge_id().to_vec();
        let labels = [("executable_checksum_last_8".to_string(), last_8), id.get(0).cloned().expect("id")];
        gauge!("redgold.node.executable_checksum_last_8", &labels).set(1.0);
    }

    fn load_mnemonic(&mut self) -> Result<(), ErrorInfo> {

        // this step probably removable?
        let mut config_data = (*self.node_config.config_data).clone();
        let node = config_data.node.get_or_insert(Default::default());
        // Remove any defaults; we want to be explicit
        node.words = Some("".to_string());

        // First try to load from the all environment data folder for re-use across environments
        if let Ok(words) = self.node_config.data_folder.all().mnemonic_no_tokio() {
            node.words = Some(words);
        };

        // Then override with environment specific mnemonic;
        if let Ok(words) = self.node_config.env_data_folder().mnemonic_no_tokio() {
            node.words = Some(words);
        };


        // TODO: Change this to write to the config

        // If empty, generate a new mnemonic;
        if node.words.as_ref().expect("w").is_empty() {
            if let Some(dbg_id) = self.node_config.debug_id().as_ref() {
                node.words = Some(WordsPass::from_str_hashed(dbg_id.to_string()).words);
            } else {
                tracing::info!("Unable to load mnemonic for wallet / node keys, attempting to generate new one");
                tracing::info!("Generating with entropy for 24 words, process may halt if insufficient entropy on system");
                let mnem = WordsPass::generate()?.words;
                tracing::info!("Successfully generated new mnemonic");
                node.words = Some(mnem.clone());
                let buf = self.node_config.env_data_folder().mnemonic_path();
                fs::write(
                    buf.clone(),
                    mnem.clone()
                ).expect("Unable to write mnemonic to file");

                info!("Wrote mnemonic to path: {}", buf.to_str().expect("Path format failure"));
            }
        };

        self.node_config.config_data = Arc::new(config_data);

        // Validate that this is loadable
        let _ = WordsPass::words(self.node_config.mnemonic_words().clone()).mnemonic()?;

        Ok(())
    }

    // TODO: Load merkle tree of this
    fn load_peer_id(&mut self) -> Result<(), ErrorInfo> {

        if let Some(p) = fs::read_to_string(self.node_config.env_data_folder().peer_id_path()).ok() {
            self.node_config.peer_id = PeerId::from_hex(p)?;
        }

        if self.node_config.peer_id.peer_id.is_none() {
            tracing::trace!("No peer_id found, attempting to generate a single key peer_id from existing mnemonic");
            // let string = self.node_config.mnemonic_words.clone();
            // TODO: we need to persist the merkle tree here as json or something
            // let tree = crate::node_config::peer_id_from_single_mnemonic(string)?;
            self.node_config.peer_id = self.node_config.default_peer_id()?;
        }

        trace!("Starting with peer id {}", self.node_config.peer_id.hex());

        Ok(())

    }

    fn data_folder(&mut self) -> Result<(), ErrorInfo> {

        let mut data_folder_path =  self.node_config.config_data.data.clone()
            .map(|p| PathBuf::from(p))
            .unwrap_or(get_default_data_top_folder());

        // Testing only modification, could potentially do this in a separate function to
        // unify this with other debug mods.
        if let Some(id) = self.opts().debug_args.debug_id {
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
        if let Some(dbg_id) = self.opts().debug_args.debug_id {
            self.node_config.port_offset = Self::debug_id_port_offset(
                self.node_config.network.default_port_offset(),
                dbg_id
            );
        }
    }

    pub fn debug_id_port_offset(offset: u16, debug_id: i32) -> u16 {
        offset + ((debug_id * 1000) as u16)
    }

    // pub fn parse_seed(&mut self) {
    //     if let Some(a) = &self.opts().seed_address {
    //         let default_port = self.node_config.network.default_port_offset();
    //         let port = self.opts().seed_port_offset.map(|p| p as u16).unwrap_or(default_port);
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
        let log_level = &self.opts().global_settings.log_level
            .clone()
            .and(std::env::var("REDGOLD_LOG_LEVEL").ok())
            .unwrap_or("DEBUG".to_string());
        let mut enable_logger = false;

        if let Some(sc) = self.get_subcommand() {
            enable_logger = match sc {
                RgTopLevelSubcommand::GUI(_) => { true }
                RgTopLevelSubcommand::Node(_) => { true }
                // RgTopLevelSubcommand::TestTransaction(_) => {true}
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
        self.node_config.network = match &self.opts().global_settings.network {
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

        if self.is_gui() && self.node_config.network == NetworkEnvironment::Local {
            if self.node_config.development_mode() {
                self.node_config.network = NetworkEnvironment::Dev;
            } else {
                self.node_config.network = NetworkEnvironment::Main;
            }
        }

        if self.node_config.network == NetworkEnvironment::Local || self.node_config.network == NetworkEnvironment::Debug {
            self.node_config.disable_auto_update = true;
            self.node_config.load_balancer_url = "127.0.0.1".to_string();
        }
        Ok(())
    }

    async fn configure_seeds(&mut self) {

        // info!("configure seeds");
        let seeds = get_seeds_by_env_time(&self.node_config.network, util::current_time_millis_i64());
        for seed in seeds {
            if !self.node_config.ignore_default_seeds {
                self.node_config.seeds.push(seed);
            }
        }

        let port = self.node_config.public_port();

        if let Some(a) = &self.opts().debug_args.seed_address {
            let default_port = self.node_config.network.default_port_offset();
            let port = self.opts().debug_args.seed_port_offset.map(|p| p as u16).unwrap_or(default_port);
            info!("Adding seed from command line arguments {a}:{port}");
            // TODO: replace this with the other seed class.
            let cli_seed_arg = Seed {
                external_address: a.clone(),
                environments: vec![self.node_config.network as i32],
                port_offset: Some(port as u32),
                trust: vec![TrustData::from_label(1.0)],
                peer_id: None, // Some(self.node_config.peer_id()),
                public_key: None, //Some(self.node_config.public_key()),
            };
            self.node_config.seeds.push(cli_seed_arg.clone());
            self.node_config.manually_added_seeds.push(cli_seed_arg);
        }

        // info!("self is node");
        // Enrich keys for missing seed info
        if self.is_node() {
            for seed in self.node_config.seeds.iter_mut() {
                if seed.public_key.is_none() {
                    info!("Querying seed: {}", seed.external_address.clone());

                    let response = RgHttpClient::new(
                        seed.external_address.clone(),
                                                     port, // TODO: Account for seed listed offset instead of direct.
                                                     // seed.port_offset.map(|p| (p + 1) as u16)
                                                     //     .unwrap_or(port),
                                                     None
                    ).about().await;
                    if let Ok(response) = response {
                        info!("Seed enrichment response issue: {}", response.json_or());
                        let nmd = response.peer_node_info.as_ref()
                            .and_then(|n| n.latest_node_transaction.as_ref())
                            .and_then(|n| n.node_metadata().ok());
                        let pk = nmd.as_ref().and_then(|n| n.public_key.as_ref());
                        let pid = nmd.as_ref().and_then(|n| n.peer_id.as_ref());
                        if let (Some(pk), Some(pid)) = (pk, pid) {
                            info!("Enriched seed {} public {} peer id {}", seed.external_address.clone(), pk.json_or(), pid.json_or());
                            seed.public_key = Some(pk.clone());
                            seed.peer_id = Some(pid.clone());
                        }
                    }
                }
            }
        }
        // info!("Seeds: {:?}", self.node_config.seeds.json_or());
        // We should enrich this too
        // TODO: Test config should pass ids so we get ids for local_test



    }

    fn genesis(&mut self) {
        if self.node_config.genesis() {
            self.node_config.seeds.push(self.node_config.self_seed())
        }
    }

    fn args(&self) -> Vec<&String> {
        // First argument is the executable path
        self.node_config.args.iter().dropping(1).collect_vec()
    }

    fn set_gui_on_empty(&mut self) {
        // println!("args: {:?}", self.args.clone());

        if self.node_config.args.len() == 1 || self.get_subcommand().is_none() {
            self.determined_subcommand = Some(RgTopLevelSubcommand::GUI(GUI{}));
        }

    }
    fn set_public_key(&mut self) {
        let pk = self.node_config.words().default_public_key().unwrap();
        self.node_config.public_key = pk.clone();
        info!("Public key: {}", pk.hex());
    }
    fn secure_data_folder(&mut self) {
        if let Some(pb) = Self::secure_data_path_buf() {
            let pb_joined = pb.join(".rg");
            self.node_config.secure_data_folder = Some(DataFolder::from_path(pb_joined));
        }
    }
    fn immediate_debug(&self) {
        if let Some(cmd) = self.get_subcommand() {
            match cmd {
                // RgTopLevelSubcommand::TestCapture(t) => {
                //     println!("Attempting test capture");
                //     #[cfg(target_os = "linux")] {
                //         use redgold_gui::image_capture_openpnp::debug_capture;
                //         debug_capture(t.cam);
                //         unsafe {
                //             exit(0)
                //         }
                //     }
                // }
                _ => {}
            }
        }
    }
    fn load_peer_tx(&self) -> RgResult<()> {
        Ok(())
    }
}


/**
This function uses an external program for calculating checksum.
Tried doing this locally, but for some reason it seemed to have a different output than the shell script.
There's internal libraries for getting the current exe path and calculating checksum, but they
seem to produce a different result than the shell script.
*/
fn calc_sha_sum(path: String) -> RgResult<String> {
    redgold_common_no_wasm::cmd::run_cmd_safe("shasum", vec!["-a", "256", &*path])
        .and_then(|x|
            x.0
             .split_whitespace()
             .next()
                .ok_or(error_info("No output from shasum"))
                .map(|x| x.to_string())
        )
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
    println!("{:?}", calc_sha_sum("Cargo.toml".to_string()));
}

#[test]
fn load_ds_path() {
    let _config = NodeConfig::default();
    // let res = load_node_config_initial(args::empty_args(), config);
    // println!("{}", res.data_store_path());
}

