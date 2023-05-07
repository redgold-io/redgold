use std::path::PathBuf;
use bitcoin_wallet::account::MasterKeyEntropy;
use bitcoin_wallet::mnemonic::Mnemonic;
use clap::command;
use serde_json::error::Category::Data;
use redgold_data::DataStoreContext;
use redgold_schema::structs::{Address, ErrorInfo, Hash, NetworkEnvironment, TransactionAmount};
use redgold_schema::structs::HashType::Transaction;
use redgold_schema::{json, json_from, json_pretty, SafeOption, util};
use redgold_schema::servers::Server;
use redgold_schema::transaction_builder::TransactionBuilder;
use crate::data::data_store::{DataStore, MnemonicEntry};
use crate::node_config::NodeConfig;
use crate::util::cli::arg_parse_config::get_default_data_directory;
use crate::util::cli::args::{AddServer, Deploy, FaucetCli, GenerateMnemonic, QueryCli, WalletAddress, WalletSend};
use crate::util::cmd::run_cmd;

pub async fn add_server(add_server: &AddServer, config: &NodeConfig) -> Result<(), ErrorInfo>  {
    let ds = config.data_store().await;
    let config = ds.config_store.select_config("servers".to_string()).await?;
    let mut servers: Vec<Server> = vec![];
    if let Some(s) = config {
        servers = json_from::<Vec<Server>>(&*s)?;
    }
    let max_index = servers.iter().map(|s| s.index).max().unwrap_or(-1);
    let this_index = add_server.index.unwrap_or(max_index + 1);
    servers.push(Server{
        host: add_server.host.clone(),
        username: add_server.user.clone(),
        key_path: add_server.key_path.clone(),
        index: this_index,
        peer_id_index: add_server.peer_id_index.unwrap_or(this_index),
        network_environment: NetworkEnvironment::All,
    });
    ds.config_store.insert_update("servers".to_string(), json(&servers)?).await?;
    Ok(())
}

pub async fn status(config: &NodeConfig) -> Result<(), ErrorInfo>  {
    //let ds = config.data_store().await;
    let c = config.lb_client();
    let a = c.about().await?;
    println!("{}", json_pretty(&a)?);

    let m = c.metrics().await?;

    println!("metrics: {}", m);

    Ok(())
}

#[ignore]
#[tokio::test]
async fn status_debug() {
    let mut nc = NodeConfig::default();
    nc.network = NetworkEnvironment::Predev;
    status(&nc).await.ok();

}

pub async fn list_servers(config: &NodeConfig) -> Result<Vec<Server>, ErrorInfo>  {
    let ds = config.data_store().await;
    let config = ds.config_store.select_config("servers".to_string()).await?;
    let mut servers: Vec<Server> = vec![];
    if let Some(s) = config {
        servers = json_from::<Vec<Server>>(&*s)?;
    }
    Ok(servers)
}

pub fn generate_mnemonic(generate_mnemonic: &GenerateMnemonic) {
    let m = generate_random_mnemonic();
    println!("{}", m.to_string());
}

pub fn generate_address(generate_address: WalletAddress, node_config: &NodeConfig) {
    let wallet = node_config.wallet();
    let address = if let Some(path) = generate_address.path {
        wallet.keypair_from_path_str(path).address_typed()
    } else if let Some(index) = generate_address.address {
        wallet.key_at(index as usize).address_typed()
    } else {
        node_config.wallet().active_keypair().address_typed()
    };
    println!("{}", address.render_string().expect("address render failure"));
}

pub async fn send(p0: &WalletSend, p1: &NodeConfig) -> Result<(), ErrorInfo> {
    let destination = Address::parse(p0.to.clone())?;
    let kp = p1.wallet().active_keypair();
    let client = p1.lb_client();
    let result = client.query_address(vec![kp.address_typed()]).await?.as_error()?;
    let utxos = result.query_addresses_response.safe_get_msg("missing query_addresses_response")?
        .utxo_entries.clone();
    // TODO ^ Balance check here
    if utxos.len() == 0 {
        return Err(ErrorInfo::error_info("No UTXOs found for this address"));
    }

    let utxo = utxos.get(0).expect("first").clone();
    let b = TransactionBuilder::new()
        .with_input(utxo, kp)
        .with_output(&destination, TransactionAmount::from_fractional(p0.amount)?)
        .with_remainder()
        .transaction.clone();

    let response = client.send_transaction(&b, false).await?.as_error()?;
    let tx_hex = response.submit_transaction_response.expect("response").transaction_hash.expect("hash").hex();
    println!("{}", tx_hex);
    Ok(())
}

pub async fn faucet(p0: &FaucetCli, p1: &NodeConfig) -> Result<(), ErrorInfo>  {
    let address = Address::parse(p0.to.clone())?;
    let response = p1.lb_client().faucet(&address, false).await?;
    let tx = response.transaction.safe_get()?;
    let tx_hex = tx.hash.safe_get()?.hex();
    println!("{}", tx_hex);
    Ok(())
}


pub async fn query(p0: &QueryCli, p1: &NodeConfig) -> Result<(), ErrorInfo> {
    let response = p1.lb_client().query_hash(p0.hash.clone()).await?;
    Ok(())
}

pub fn mnemonic_fingerprint(m: Mnemonic) -> String {
    let vec = m.to_seed(None).0;
    let res = Hash::calc_bytes(vec);
    let vec2 = res.vec();
    let vec1 = vec2[0..4].to_vec();
    let hx = hex::encode(vec1);
    hx
}

pub fn generate_random_mnemonic() -> Mnemonic {
    Mnemonic::new_random(MasterKeyEntropy::Paranoid)
        .expect("New mnemonic generation failure")
}

pub const REDGOLD_SECURE_DATA_PATH: &str = "REDGOLD_SECURE_DATA_PATH";

pub fn default_path() -> PathBuf {
    dirs::home_dir().expect("home directory not found").join(".rg")
}

pub fn default_all_path() -> String {
    default_path().join("all").to_str().expect("all").to_string()
}

pub fn add_server_prompt() -> Server {
    println!("Enter hostname or IP address of server");
    let mut host = String::new();
    std::io::stdin().read_line(&mut host).expect("Failed to read line");
    println!("Enter SSH username, or press enter to use default of 'root'");
    let mut username = "root".to_string();
    std::io::stdin().read_line(&mut username).expect("Failed to read line");
    println!("Enter SSH key path, or press enter to use default of ~/.ssh/id_rsa");
    let mut key_path = dirs::home_dir().expect("Home").join(".ssh/id_rsa").to_str().expect("str").to_string();
    std::io::stdin().read_line(&mut key_path).expect("Failed to read line");
    Server{
        host,
        key_path: Some(key_path),
        index: 0,
        username: Some(username),
        peer_id_index: 0,
        network_environment: NetworkEnvironment::All,
    }
}

pub async fn deploy(deploy: Deploy, config: NodeConfig) -> Result<(), ErrorInfo> {

    if deploy.wizard {
        println!("Welcome to the Redgold deployment wizard!");
        let mut data_dir = get_default_data_directory(NetworkEnvironment::All);
        let path = std::env::var(REDGOLD_SECURE_DATA_PATH);
        match path {
            Ok(p) => {
                println!("Found secure data path: {}", p);
            }
            Err(_) => {
                println!("No secure data path found, please enter a path to store secure data");
                println!("This should ideally be an encrypted volume (Cryptomator or equivalent) \
                with cloud backups (pCloud or equivalent)");
                println!("If you are unsure, press enter without a path to use the default path");
                let mut path = String::new();
                std::io::stdin().read_line(&mut path).expect("Failed to read line");
                if path.is_empty() {
                    let buf = data_dir.clone();
                    println!("Using default path {}", buf.to_str().expect("").to_string());
                } else {
                    println!("Using path: {}", path);
                    println!("Would you like to add to ~/.bash_profile (y/n)");
                    let mut answer = String::new();
                    std::io::stdin().read_line(&mut answer).expect("Failed to read line");
                    if answer.trim().to_lowercase() == "y" {
                        println!("Adding path to ~/.bash_profile");
                        let (stdout, stderr) = run_cmd(
                            "echo", vec![&format!("'export REDGOLD_SECURE_DATA_PATH={}'", path), ">>", "~/.bash_profile"]);
                        println!("{} {}", stdout, stderr);
                    }
                    data_dir = PathBuf::from(path).join("all");
                }
            }
        }


        // Query to find if any existing servers
        let store_path = data_dir.join("data_store.sqlite").to_str().expect("str").to_string();
        let ds = DataStore::from_path(store_path).await;

        // Check to see if we have a mnemonic stored in backup for generating a random seed

        let mnemonics = DataStoreContext::map_err_sqlx(ds.query_all_mnemonic().await)?;
        let mnemonic = if mnemonics.is_empty() {
            println!("Unable to find random mnemonic from backup, generating a new one and saving");
            let m = generate_random_mnemonic();
            DataStoreContext::map_err_sqlx(ds.insert_mnemonic(MnemonicEntry{
                words: m.to_string(),
                time: util::current_time_millis(),
                peer_id: vec![]
            }).await)?;
            m
        } else {
            println!("Found stored random mnemonic");
            let x = mnemonics.get(0).expect("").clone();
            let string = x.words.clone();
            Mnemonic::from_str(&string).expect("words")
        };
        println!("Random mnemonic fingerprint: {}", mnemonic_fingerprint(mnemonic.clone()));

        let mut servers: Vec<Server> = vec![]; // ds.server_store.servers().await?;

        if servers.is_empty() {
            println!("No deployment server found, please add a new server");
            let server = add_server_prompt();
            servers.push(server);
        }

        let mut done_adding_servers = false;
        while !done_adding_servers {
            println!("Would you like to add another server? (y/n)");
            let mut answer = String::new();
            std::io::stdin().read_line(&mut answer).expect("Failed to read line");
            if answer.trim().to_lowercase() == "y" {
                let server = add_server_prompt();
                servers.push(server);
            } else {
                done_adding_servers = true;
            }
        }

        println!("Enter deployment target environment: 'main' for mainnet, 'test' for testnet,\
         'all' for all environments on same machine no quotes -- empty for default of 'all'");
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer).expect("Failed to read line");
        let network_env = if answer.is_empty(){
            NetworkEnvironment::All
        } else { NetworkEnvironment::parse(answer.trim().to_lowercase())};

        println!("Enter mnemonic passphrase, leave empty for none");
        let mut passphrase_input = String::new();
        std::io::stdin().read_line(&mut answer).expect("Failed to read line");
        let passphrase: Option<&str> = if passphrase_input.is_empty() {
            None
        } else {
            Some(&*passphrase_input)
        };
        let seed_bytes = mnemonic.to_seed(passphrase).0;

        for server in servers {

        }


    }
    Ok(())

}
