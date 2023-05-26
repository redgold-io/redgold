use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use bitcoin_wallet::account::MasterKeyEntropy;
use bitcoin_wallet::mnemonic::Mnemonic;
use clap::command;
use log::info;
use rocket::form::FromForm;
use serde_json::error::Category::Data;
use tokio::runtime::Runtime;
use redgold_data::DataStoreContext;
use redgold_schema::structs::{Address, ErrorInfo, Hash, NetworkEnvironment, Proof, PublicKey, TransactionAmount};
use redgold_schema::structs::HashType::Transaction;
use redgold_schema::{error_info, json, json_from, json_pretty, KeyPair, SafeOption, util};
use redgold_schema::servers::Server;
use redgold_schema::transaction::{rounded_balance, rounded_balance_i64};
use redgold_schema::transaction_builder::TransactionBuilder;
use crate::canary::tx_submit::TransactionSubmitter;
use crate::data::data_store::{DataStore, MnemonicEntry};
use crate::node_config::NodeConfig;
use crate::util::cli::arg_parse_config::get_default_data_directory;
use crate::util::cli::args::{AddServer, BalanceCli, Deploy, FaucetCli, GenerateMnemonic, QueryCli, TestTransactionCli, WalletAddress, WalletSend};
use crate::util::cmd::run_cmd;
use redgold_schema::EasyJson;
use crate::node::NodeRuntimes;
use crate::util::init_logger;
use crate::util::runtimes::build_runtime;

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
    nc.network = NetworkEnvironment::Dev;
    status(&nc).await.expect("");

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

pub fn generate_address(generate_address: WalletAddress, node_config: &NodeConfig) -> Result<String, ErrorInfo> {
    let wallet = node_config.internal_mnemonic();
    let address = if let Some(path) = generate_address.path {
        wallet.keypair_from_path_str(path).address_typed()
    } else if let Some(index) = generate_address.index {
        wallet.key_at(index as usize).address_typed()
    } else {
        node_config.internal_mnemonic().active_keypair().address_typed()
    };
    let string = address.render_string().expect("address render failure");
    println!("{}", string.clone());
    Ok(string)
}




pub async fn send(p0: &WalletSend, p1: &NodeConfig) -> Result<(), ErrorInfo> {
    let destination = Address::parse(p0.to.clone())?;
    let mut query_addresses = vec![];
    let mut hm: HashMap<Vec<u8>, KeyPair> = HashMap::new();
    // for x in p0.from {
    //     let address = Address::parse(x)?;
    //     query_addresses.push(address);
    // }
    use redgold_schema::SafeBytesAccess;

    for i in 0..10 {
        let kp = p1.internal_mnemonic().key_at(i as usize);
        let x1 = kp.address_typed();
        let x: Vec<u8> = x1.address.safe_bytes()?;
        query_addresses.push(x1);
        hm.insert(x, kp.clone());
    }

    let client = p1.lb_client();
    let result = client.query_address(query_addresses).await?.as_error()?;
    let utxos = result.query_addresses_response.safe_get_msg("missing query_addresses_response")?
        .utxo_entries.clone();
    // TODO ^ Balance check here
    if utxos.len() == 0 {
        return Err(ErrorInfo::error_info("No UTXOs found for this address"));
    }
    let option = hm.get(&utxos.get(0).safe_get_msg("first")?.address);
    let kp = option.safe_get_msg("keypair")?.clone().clone();

    let utxo = utxos.get(0).expect("first").clone();
    let b = TransactionBuilder::new()
        .with_input(utxo, kp)
        .with_output(&destination, TransactionAmount::from_fractional(p0.amount)?)
        .with_remainder()
        .transaction.clone();

    let response = client.send_transaction(&b, false).await?;
    let tx_hex = response.transaction_hash.expect("hash").hex();
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

pub async fn balance_lookup(request: &BalanceCli, nc: &NodeConfig) -> Result<(), ErrorInfo> {
    let response = nc.lb_client().query_hash(request.address.clone()).await?;
    let rounded = rounded_balance_i64(response.address_info.safe_get_msg("missing address_info")?.balance);
    println!("{}", rounded.to_string());
    Ok(())
}


pub async fn query(p0: &QueryCli, p1: &NodeConfig) -> Result<(), ErrorInfo> {
    let response = p1.lb_client().query_hash(p0.hash.clone()).await?;
    println!("{}", json_pretty(&response)?);
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

pub async fn test_transaction(p0: &&TestTransactionCli, p1: &NodeConfig
                        // , arc: Arc<Runtime>
) -> Result<(), ErrorInfo> {
    if p1.network == NetworkEnvironment::Main {
        return Err(error_info("Cannot test transaction on mainnet unsupported".to_string()));
    }
    let client = p1.lb_client();
    let mut tx_submit = TransactionSubmitter::default(client.clone(),
                                                      // arc.clone(),
                                                      vec![]
    );
    let faucet_tx = tx_submit.with_faucet().await?;
    // info!("Faucet response: {}", faucet_tx.json_or());
    let faucet_tx = faucet_tx.transaction.safe_get()?;
    let _ = {
        let gen =
        tx_submit.generator.lock().expect("");
        assert!(gen.finished_pool.len() > 0);
    };
    let source = Proof::proofs_to_address(&faucet_tx.inputs.get(0).expect("").proof)?;
    let repeat = tx_submit.drain(source).await?;
    // assert!(repeat.accepted());
    // assert proofs here
    let s = repeat;
    // info!("Repeat response: {}", s.json_or());
    // let h2 = s.transaction_hash.expect("hash");
    let q = s.query_transaction_response.expect("query transaction response");
    // println!("Obs proofs second tx: {}", q.observation_proofs.json_or());
    let i = q.observation_proofs.len();
    println!("Obs proofs number length: {:?}", i);
    assert!(i > 0);
    let mut peer_keys: HashSet<PublicKey> = HashSet::new();

    for o in q.observation_proofs {
        let key = o.proof.expect("p").public_key.expect("");
        peer_keys.insert(key);
    }

    println!("Number of unique peer observations {}", peer_keys.len());

    // client.client_wrapper()



    // let result = arc.block_on(client.query_hash(h2.hex())).expect("query hash");
    // info!("Result: {}", result.json_pretty().expect("json pretty"));
    Ok(())

}

#[ignore]
#[tokio::test]
async fn test_transaction_dev() {
    init_logger();
    let mut nc = NodeConfig::default();
    nc.network = NetworkEnvironment::Dev;
    // let rt = build_runtime(5, "asdf");
    let t = TestTransactionCli{};
    // let arc = rt.clone();
    let res = test_transaction(&&t, &nc
                               // , arc
    ).await.expect("");
}