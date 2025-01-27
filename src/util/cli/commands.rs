use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;
use flume::Sender;
use itertools::Itertools;

use tracing::{error, info};
use rocket::form::FromForm;
use tokio::task::JoinHandle;
use redgold_common::flume_send_help::{Channel, RecvAsyncErrorInfo};
use redgold_data::data_store::DataStore;
use redgold_keys::address_support::AddressSupport;
use redgold_keys::KeyPair;
use redgold_keys::proof_support::ProofSupport;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::btc::btc_wallet::SingleKeyBitcoinWallet;
use redgold_schema::keys::words_pass::WordsPass;
use redgold_schema::{error_info, ErrorInfoContext, RgResult, SafeOption};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::conf::rg_args::{AddServer, BalanceCli, DebugCommand, Deploy, FaucetCli, GenerateMnemonic, QueryCli, RgDebugCommand, TestTransactionCli, WalletAddress, WalletSend};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::easy_json::{json, json_from, json_pretty};
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::servers::ServerOldFormat;
use redgold_schema::structs::{Address, CurrencyAmount, ErrorInfo, Hash, NetworkEnvironment, Proof, PublicKey};
use redgold_schema::transaction::rounded_balance_i64;
use redgold_schema::tx::tx_builder::TransactionBuilder;
use crate::core::relay::Relay;
use crate::e2e::tx_submit::TransactionSubmitter;
use crate::infra::deploy::default_deploy;
use crate::infra::grafana_public_manual_deploy::manual_deploy_grafana_public;
use crate::node_config::{ApiNodeConfig, DataStoreNodeConfig, EnvDefaultNodeConfig};
use redgold_common_no_wasm::cmd::run_cmd;
use redgold_common_no_wasm::output_handlers;
use redgold_common_no_wasm::ssh_like::ToSSHDeploy;
use redgold_common_no_wasm::tx_new::TransactionBuilderSupport;
use redgold_keys::util::mnemonic_support::MnemonicSupport;
use redgold_keys::word_pass_support::{NodeConfigKeyPair, WordsPassNodeConfig};
use redgold_schema::observability::errors::Loggable;
use crate::core::backup::aws_backup::AwsBackup;
use crate::core::transact::tx_broadcast_support::TxBroadcastSupport;
use crate::core::transact::tx_builder_supports::{TxBuilderApiConvert, TxBuilderApiSupport};
use crate::test::daily_e2e::run_daily_e2e;
use crate::util::metadata::read_metadata_json;

pub async fn add_server(add_server: &AddServer, config: &NodeConfig) -> Result<(), ErrorInfo>  {
    let ds = config.data_store().await;
    let config = ds.config_store.select_config("servers".to_string()).await?;
    let mut servers: Vec<ServerOldFormat> = vec![];
    if let Some(s) = config {
        servers = json_from::<Vec<ServerOldFormat>>(&*s)?;
    }
    let max_index = servers.iter().map(|s| s.index).max().unwrap_or(-1);
    let this_index = add_server.index.unwrap_or(max_index + 1);
    servers.push(ServerOldFormat {
        name: "".to_string(),
        host: add_server.ssh_host.clone(),
        username: add_server.user.clone(),
        ipv4: None,
        node_name: None,
        index: this_index,
        peer_id_index: add_server.peer_id_index.unwrap_or(this_index),
        network_environment: NetworkEnvironment::All.to_std_string(),
        external_host: None,
        reward_address: None,
        jump_host: None,
        party_config: None,
    });
    ds.config_store.insert_update("servers".to_string(), json(&servers)?).await?;
    Ok(())
}

pub async fn status(config: &NodeConfig) -> Result<(), ErrorInfo>  {
    //let ds = config.data_store().await;
    let c = config.api_rg_client();
    let a = c.about().await?;
    println!("{}", json_pretty(&a)?);

    let m = c.metrics().await?.json_or();

    println!("metrics: {}", m);

    Ok(())
}

#[ignore]
#[tokio::test]
async fn metrics_debug() {
    let mut nc = NodeConfig::default();
    nc.network = NetworkEnvironment::Dev;
    let c = nc.api_rg_client();
    // let a = c.about().await?;
    // println!("{}", json_pretty(&a)?);

    let m = c.metrics().await.expect("");
    let t = c.table_sizes().await.expect("");

    println!("metrics: {}", m.json_or());

    println!("table sizes: {}", t.json_or());


}

pub async fn list_servers(config: &NodeConfig) -> Result<Vec<ServerOldFormat>, ErrorInfo>  {
    let ds = config.data_store().await;
    let config = ds.config_store.select_config("servers".to_string()).await?;
    let mut servers: Vec<ServerOldFormat> = vec![];
    if let Some(s) = config {
        servers = json_from::<Vec<ServerOldFormat>>(&*s)?;
    }
    Ok(servers)
}

pub fn generate_mnemonic(_generate_mnemonic: &GenerateMnemonic) -> RgResult<()> {
    let wp = WordsPass::generate().expect("works");
    println!("{}", wp.words);
    Ok(())
}

pub fn generate_address(generate_address: WalletAddress, node_config: &NodeConfig) -> Result<String, ErrorInfo> {
    let wallet = node_config.secure_words_or();
    let address = if let Some(path) = generate_address.path {
        wallet.keypair_at(path).expect("works").address_typed()
    } else if let Some(index) = generate_address.index {
        wallet.keypair_at_change(index as i64).expect("works").address_typed()
    } else {
        node_config.keypair().address_typed()
    };
    let string = address.render_string().expect("address render failure");
    println!("{}", string.clone());
    Ok(string)
}




pub async fn send(p0: &WalletSend, p1: &NodeConfig) -> Result<(), ErrorInfo> {
    let destination = p0.destination.parse_address()?;
    let amount = p0.amount;
    let amount = CurrencyAmount::from_fractional(amount)?;
    // let mut query_addresses = vec![];
    // let mut hm: HashMap<Vec<u8>, KeyPair> = HashMap::new();
    // for x in p0.from {
    //     let address = Address::parse(x)?;
    //     query_addresses.push(address);
    // }
    let kp = p1.secure_words_or().keypair_at_change(0 as i64).expect("works");
    //
    // for i in 0..10 {
    //     let x1 = kp.address_typed();
    //     let x: Vec<u8> = x1.vec();
    //     query_addresses.push(x1);
    //     hm.insert(x, kp.clone());
    // }
    //
    let client = p1.api_client();
    // let result = client.query_address(query_addresses).await?.as_error()?;
    // let utxos = result.query_addresses_response.safe_get_msg("missing query_addresses_response")?
    //     .utxo_entries.clone();
    // // TODO ^ Balance check here
    // if utxos.len() == 0 {
    //     return Err(ErrorInfo::error_info("No UTXOs found for this address"));
    // }
    // let option1 = utxos.get(0);
    // let first_uto = option1.safe_get_msg("first")?;
    // let first_addr = first_uto.address()?;
    // let option = hm.get(&first_addr.vec());
    // let kp = option.safe_get_msg("keypair")?.clone().clone();
    //
    // let utxo = utxos.get(0).expect("first").clone();
    let b = TransactionBuilder::new(&p1)
        .with_input_address(&kp.address_typed())
        .into_api_wrapper()
        .with_auto_utxos().await?
        .with_output(&destination, &amount)
        .build()?
        .sign(&kp)?;

    let response = client.send_transaction(&b, true).await?;
    let tx_hex = response.transaction_hash.safe_get()?.hex();
    println!("{}", tx_hex);
    Ok(())
}

pub async fn faucet(p0: &FaucetCli, p1: &NodeConfig) -> Result<(), ErrorInfo>  {
    let address = p0.to.clone().parse_address()?;
    let response = p1.api_client().faucet(&address).await?;
    let tx = response.submit_transaction_response.safe_get()?.transaction.safe_get()?;
    let tx_hex = tx.hash_hex();
    println!("{}", tx_hex);
    Ok(())
}

pub async fn balance_lookup(request: &BalanceCli, nc: &Box<NodeConfig>) -> Result<(), ErrorInfo> {
    // TODO: Get keypair from prior cli steps.
    let w = nc.secure_words_or().keypair_at_change(0).expect("works");
    let addr = if let Some(a) = request.address.as_ref() {
        a.parse_address()?
    } else {
        w.address_typed()
    };
    // println!("about to query");
    let response = nc.api_client().query_hash(addr.render_string()?).await?;
    let rounded = rounded_balance_i64(response.address_info.safe_get_msg("missing address_info")?.balance);
    println!("{}", rounded.to_string());
    Ok(())
}


pub async fn query(p0: &QueryCli, p1: &Box<NodeConfig>) -> Result<(), ErrorInfo> {
    let h = p0.hash.clone();
    let response = p1.api_client().query_hash(h.clone()).await?;
    println!("{}", json(&response)?);
    Ok(())
}


pub fn generate_random_mnemonic() -> WordsPass {
    WordsPass::generate().expect("works")
}


#[test]
pub fn mnemonic_generate_test() {
    assert_eq!(generate_random_mnemonic().words.split(" ").count(), 24);
}

pub const REDGOLD_SECURE_DATA_PATH: &str = "REDGOLD_SECURE_PATH";

pub fn default_path() -> PathBuf {
    dirs::home_dir().expect("home directory not found").join(".rg")
}

pub fn default_all_path() -> String {
    default_path().join("all").to_str().expect("all").to_string()
}

pub fn add_server_prompt() -> ServerOldFormat {
    println!("Enter hostname or IP address of server");
    let mut host = String::new();
    std::io::stdin().read_line(&mut host).expect("Failed to read line");
    println!("Enter SSH username, or press enter to use default of 'root'");
    let mut username = "root".to_string();
    std::io::stdin().read_line(&mut username).expect("Failed to read line");
    println!("Enter SSH key path, or press enter to use default of ~/.ssh/id_rsa");
    let mut key_path = dirs::home_dir().expect("Home").join(".ssh/id_rsa").to_str().expect("str").to_string();
    std::io::stdin().read_line(&mut key_path).expect("Failed to read line");
    ServerOldFormat {
        name: "".to_string(),
        host,
        index: 0,
        username: Some(username),
        ipv4: None,
        node_name: None,
        peer_id_index: 0,
        network_environment: NetworkEnvironment::All.to_std_string(),
        external_host: None,
        reward_address: None,
        jump_host: None,
    }
}

pub async fn deploy(deploy: &Deploy, node_config: &NodeConfig) -> RgResult<JoinHandle<()>> {
    let mut deploy = deploy.clone();
    // if deploy.wizard {
    //     deploy_wizard(&deploy, node_config).await?;
    //     return Ok(tokio::spawn(async move {()}));
    // }

    if std::env::var("REDGOLD_PRIMARY_GENESIS").is_ok() {
        deploy.genesis = true;
    }
    let mut net = node_config.network.clone();

    if net == NetworkEnvironment::Local {
        net = NetworkEnvironment::Dev;
    } else {
        if node_config.config_data.network.is_none() {
            if node_config.development_mode() {
                net = NetworkEnvironment::Dev;
            } else {
                net = NetworkEnvironment::Main;
            }
        }
        // Get node_config arg translate and set to dev if arg not supplied.
    }

    let mut nc = node_config.clone();
    nc.network = net;


    let (default_fun, output_handler) = output_handlers::log_handler();

    default_deploy(&mut deploy, &nc, output_handler, None, None).await?;

    Ok(default_fun)
}

pub async fn get_input(prompt: impl Into<String>) -> RgResult<Option<String>> {
    println!("{}", prompt.into());
    let mut input = String::new();
    // TODO: Replace with tokio async read if necessary
    std::io::stdin().read_line(&mut input).error_info("Failed to read line")?;
    if input.is_empty() {
        Ok(None)
    } else {
        Ok(Some(input))
    }
}

// TODO: All this code is out of date, need to update it.
// Move to own file
pub async fn deploy_wizard(_deploy: &Deploy, _config: &NodeConfig) -> Result<(), ErrorInfo> {

        println!("Welcome to the Redgold deployment wizard!");
        let mut data_dir = _config.data_folder.all().path;
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
                            "echo", vec![&*format!("'export REDGOLD_SECURE_PATH={}'", path), ">>", "~/.bash_profile"]);
                        println!("{} {}", stdout, stderr);
                    }
                    data_dir = PathBuf::from(path).join("all");
                }
            }
        }


        // Query to find if any existing servers
        let store_path = data_dir.join("data_store.sqlite").to_str().expect("str").to_string();
        // let _ds = DataStore::from_path(store_path, store_path).await;

        // Check to see if we have a mnemonic stored in backup for generating a random seed

        let mnemonics: Vec<WordsPass> = vec![]; // TODO: Replace from config; DataStoreContext::map_err_sqlx(ds.query_all_mnemonic().await)?;
        let mnemonic = if mnemonics.is_empty() {
            println!("Unable to find random mnemonic from backup, generating a new one and saving");
            let m = generate_random_mnemonic();
            // TODO: Replace this with updating an internal config, potentially encrypted.
            // DataStoreContext::map_err_sqlx(ds.insert_mnemonic(MnemonicEntry{
            //     words: m.to_string(),
            //     time: util::current_time_millis(),
            //     peer_id: vec![]
            // }).await)?;
            m
        } else {
            println!("Found stored random mnemonic");
            let x = mnemonics.get(0).expect("").clone();
            let string = x.words.clone();
            WordsPass::new(&string, None)
        };
        println!("Random mnemonic fingerprint: {}", mnemonic.checksum().expect("checksum"));

        let mut servers: Vec<ServerOldFormat> = vec![]; // ds.server_store.servers().await?;

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
        let _network_env = if answer.is_empty(){
            NetworkEnvironment::All
        } else { NetworkEnvironment::parse(answer.trim().to_lowercase())};

        println!("Enter mnemonic passphrase, leave empty for none");
        let passphrase_input = String::new();
        std::io::stdin().read_line(&mut answer).expect("Failed to read line");
        let passphrase: Option<&str> = if passphrase_input.is_empty() {
            None
        } else {
            Some(&*passphrase_input)
        };
        // let _seed_bytes = mnemonic.to_seed(passphrase).0;
        //
        // for server in servers {
        //
        // }
    Ok(())
}

pub async fn test_transaction(_p0: &&TestTransactionCli, p1: &NodeConfig
                        // , arc: Arc<Runtime>
) -> Result<(), ErrorInfo> {
    if p1.network == NetworkEnvironment::Main {
        return Err(error_info("Cannot test transaction on mainnet unsupported".to_string()));
    }
    let client = p1.api_client();
    let tx_submit = TransactionSubmitter::default(
        client.clone(), vec![], &p1,
    );
    let faucet_tx = tx_submit.with_faucet().await?;
    // info!("Faucet response: {}", faucet_tx.json_or());
    let faucet_tx = faucet_tx.submit_transaction_response.safe_get()?.transaction.safe_get()?;
    let address = faucet_tx.first_output_address_non_input_or_fee().expect("a");
    let response = client.query_hash(address.render_string().expect("")).await?;
    let rounded = rounded_balance_i64(response.address_info.safe_get_msg("missing address_info")?.balance);
    assert!(rounded > 0.);
    let _ = {
        let gen =
        tx_submit.generator.lock().expect("");
        assert!(gen.finished_pool.len() > 0);
    };
    let source = Proof::proofs_to_addresses(&faucet_tx.inputs.get(0).expect("").proof)?.get(0).expect("source").clone();
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
    // init_logger(); //.ok();
    let mut nc = NodeConfig::default();
    nc.network = NetworkEnvironment::Dev;
    // let rt = build_runtime(5, "asdf");
    let t = TestTransactionCli{};
    // let arc = rt.clone();
    let _ = test_transaction(&&t, &nc
                               // , arc
    ).await.expect("");
}

#[ignore]
#[tokio::test]
async fn test_new_deploy() {
    // init_logger(); //.ok();
    let nc = NodeConfig::dev_default().await;
    let mut dep = Deploy::default();
    dep.ops = false;
    info!("Deploying with {:?}", dep.clone());
    dep.server_index = Some(4);
    deploy(&dep, &nc).await.expect("works").abort();
}

pub async fn test_btc_balance(p0: &&String, network: NetworkEnvironment) {
    let hex = p0.clone().clone();
    let pk = PublicKey::from_hex_direct(&hex).expect("hex");
    let w = SingleKeyBitcoinWallet::new_wallet(pk, network, true).expect("works");
    let b = w.get_wallet_balance().expect("balance");
    println!("Balance: {:?}", b);
    info!("Balance: {:?}", b);
    // let txs = w.get_sourced_tx().expect("tx");
    // for t in txs {
    //     println!("Tx: {:?}", t);
    // }
}
pub async fn convert_metadata_xpub(path: &String) -> RgResult<()> {
    let md = read_metadata_json(path).await?;
    println!("name,derivation_path,xpub");
    for x in md.rdg_btc_message_account_metadata {
        let dp = x.derivation_path.split("/")
            .map(|x| x.replace("'", ""))
            .collect::<Vec<String>>();
        let option = dp.get(1..4);
        if let Some(strs) = option{
            let name = strs.iter().join("_");
            println!("account_{},{},{}", name, x.derivation_path.clone(), x.xpub.clone());
        }

    }
    Ok(())
}

pub async fn debug_commands(p0: &DebugCommand, p1: &Box<NodeConfig>) -> RgResult<()> {
    if let Some(cmd) = &p0.subcmd {
        match cmd {
            RgDebugCommand::GrafanaPublicDeploy(_) => {
                manual_deploy_grafana_public().await.log_error().ok();
                Ok(())
            }
            RgDebugCommand::BuildReleaseArtifacts(b) => {
                // TODO: Capture this in rust instead of bash scripts.
                /*
                cargo install cargo-bundle
                cargo bundle --release
                /target/release/bundle/osx/redgold.app
                 */
                Ok(())
            }
            RgDebugCommand::DailyTest(d) => {
                run_daily_e2e(p1).await
            }
            RgDebugCommand::S3UpDir(d) => {
                let p = PathBuf::from(d.source.clone());
                let dest = d.dest.replace("s3://", "");
                let split = dest.split("/").collect::<Vec<&str>>();
                let bucket = split.get(0).expect("bucket").to_string();
                let prefix = split.get(1).expect("prefix").to_string();
                let relay = Relay::new(*p1.clone()).await;
                AwsBackup::new(&relay).s3_upload_directory(&p, bucket, prefix).await
            }
            RgDebugCommand::CopyData(c) => {
                // for d in p1.config_data.local.as_ref().unwrap().deploy.iter() {
                //     let s = d.as_old_servers();
                //     for s in s {
                //         if s.host == c.server_ssh_host {
                //            s.to_ssh(Some(log_handler().1.expect("sender")))
                //
                //         }
                //     }
                // }
                Ok(())
            }
            _ => {
                Ok(())
            }
        }
    } else {
        Ok(())
    }
}