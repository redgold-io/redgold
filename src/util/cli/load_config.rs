use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use clap::Parser;
use config::{Config, Environment};
use itertools::Itertools;
use log::info;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::conf::rg_args::{empty_args, RgArgs};
use redgold_schema::config_data::ConfigData;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::structs::NetworkEnvironment;
use crate::util::cli::apply_args_to_config::{apply_args_final, apply_args_initial};


pub fn main_config() -> Box<NodeConfig> {
    let (opts, cfg) = load_full_config(false);
    drop(*opts);
    let mut node_config = NodeConfig::default();
    let args = std::env::args().collect_vec();
    node_config.config_data = Arc::new(*cfg.clone());
    node_config.args = Arc::new(args.clone());
    Box::new(node_config)
}

pub fn load_full_config(allow_no_args: bool) -> (Box<RgArgs>, Box<ConfigData>) {
    let rg_args = if allow_no_args {
        RgArgs::try_parse().unwrap_or(empty_args())
    } else {
        RgArgs::parse()
    };
    let default = environment_only_builder();
    let args = Box::new(rg_args);
    let init = apply_args_initial(args.clone(), default);
    let config = load_config(init);
    let config_after_final_args = apply_args_final(args.clone(), config);
    (args, config_after_final_args)
}

pub fn process_data_folder_with_env(df: Option<&String>, network: Option<String>) -> Vec<PathBuf> {
    let mut paths = vec![];
    if let Some(p) = df {
        let buf = PathBuf::from_str(p).unwrap();
        let joined = buf.join("config.toml");
        paths.push(joined);
        if let Some(n) = network {
            let net_joined = buf.join(n);
            let joined = net_joined.join("config.toml");
            paths.push(joined);
        }
    }
    paths
}

pub fn environment_only_builder() -> Box<ConfigData> {
    let mut builder = Config::builder();
    builder = builder
        .add_source(
            config_env_source(),
        );
    let config = builder
        .build()
        .unwrap();
    Box::new(config.try_deserialize::<ConfigData>().unwrap())
}

fn config_env_source() -> Environment {
    config::Environment::with_prefix("REDGOLD")
        .try_parsing(true)
        .separator("_")
        .with_list_parse_key("")
        .list_separator(",")
}

pub fn load_config(init: Box<ConfigData>) -> Box<ConfigData> {


    let mut paths: Vec<PathBuf> = vec![];

    // Start with secure path as first
    if let Some(sd) = init.secure.as_ref() {
        // First process the data folder and its environment overrides
        let path = sd.path.as_ref()
            .map(|p| PathBuf::from_str(p).unwrap().join(".rg")
                .to_str().unwrap().to_string());
        paths.extend(process_data_folder_with_env(path.as_ref(), init.network.clone()));
        // Next process a user-specified override.
        if let Some(cp) = sd.config.as_ref() {
            paths.push(PathBuf::from_str(cp).unwrap());
        }
    }

    // Next we attempt to load users home directory, this should not interfere with secure data
    // As user would need to override values manually to do so, ideally they read the docs
    // and avoid that so it's clean to merge.
    if let Some(h) = init.home.as_ref() {
        let mut home = PathBuf::from_str(h).unwrap();
        let home_df = home.join(".rg");
        let home_df = home_df.to_str().unwrap().to_string();
        paths.extend(process_data_folder_with_env(Some(&home_df), init.network.clone()));
    }

    // Next to last if a data folder was specified we repeat
    // above data folder process for non-secure data folder.
    if let Some(df) = init.data.as_ref() {
        paths.extend(process_data_folder_with_env(Some(df), init.network.clone()));
    }

    // This was specified to override all others by the user, hence last
    if let Some(cp) = init.config.as_ref() {
        paths.push(PathBuf::from_str(cp).unwrap());
    }

    let mut builder = Config::builder();

    let mut working_config_path = None;

    for p in paths.into_iter() {
        // println!("Checking if path exists: {:?}", p);
        if p.exists() {
            // println!("Loading config from: {:?}", p);
            working_config_path = Some(p.to_str().unwrap().to_string());
            builder = builder.add_source(config::File::from(p));
        }
    }

    let config = builder
        .add_source(
            config_env_source()
        )
        .build()
        .unwrap();

    // Final env override, matches config load order.
    if let Some(c) = std::env::var("REDGOLD_CONFIG").ok() {
        working_config_path = Some(c);
    }

    let mut data = config.try_deserialize::<ConfigData>().unwrap();
    // Ensure loaded config path is stored
    data.config = working_config_path;
    Box::new(data)
}

// #[ignore]
#[test]
fn debug_config_load() {
    let cfg = environment_only_builder();
    // let cfg = load_config(init);
    println!("{}", toml::to_string(&cfg).unwrap());
    println!("{}", cfg.json_or());
}


#[ignore]
#[test]
fn debug_config_load2() {
    // Set some test environment variables
    // env::set_var("REDGOLD_NETWORK", "testnet");
    env::set_var("REDGOLD_HOME", "/home/user");
    env::set_var("REDGOLD_DEBUG_TEST", "test_mnemonic");

    // Debug: Print all environment variables starting with REDGOLD
    for (key, value) in env::vars() {
        if key.starts_with("REDGOLD_") {
            println!("Debug: Found env var: {} = {}", key, value);
        }
    }

    let cfg = environment_only_builder();

    println!("Loaded config:");
    println!("{}", toml::to_string(&cfg).unwrap());
    println!("{}", cfg.json().unwrap());
    //
    // // Add some assertions
    // assert_eq!(cfg.network, Some("testnet".to_string()));
    // assert_eq!(cfg.home, Some("/home/user".to_string()));
    // assert_eq!(cfg.secure_data.as_ref().and_then(|sd| sd.salt_mnemonic.clone()),
    //            Some("test_mnemonic".to_string()));
    //
    // // Clean up
    // env::remove_var("REDGOLD_NETWORK");
    // env::remove_var("REDGOLD_HOME");
    // env::remove_var("REDGOLD_SECURE_DATA_SALT_MNEMONIC");
}

#[test]
fn test_load_full_config() {
    let (args, cfg) = load_full_config(true);
    println!("Args: {:?}", args);
    println!("Config: {}", cfg.json_or());

}