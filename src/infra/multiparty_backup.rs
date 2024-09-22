use std::path::PathBuf;
use std::time::Duration;
use log::info;
use serde::{Deserialize, Serialize};
use redgold_common_no_wasm::data_folder_read_ext::EnvFolderReadExt;
use redgold_schema::{ErrorInfoContext, from_hex, RgResult, SafeOption};
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::servers::ServerOldFormat;
use redgold_schema::structs::{InitiateMultipartyKeygenRequest, PartyInfo, PublicKey};
use crate::core::relay::Relay;
use crate::infra::deploy::DeployMachine;
use redgold_schema::conf::node_config::NodeConfig;
use crate::util;
use crate::util::cli::commands::log_handler;

pub(crate) async fn backup_multiparty_local_shares(p0: NodeConfig, p1: Vec<ServerOldFormat>) {

    let net_str = p0.network.to_std_string();
    let time = util::current_time_unix();
    let secure_or = p0.secure_or().by_env(p0.network);
    let bk = secure_or.backups();
    let time_back = bk.join(time.to_string());
    let (default_fun, output_handler) = log_handler();


    for s in p1 {
        let server_dir = time_back.join(s.index.to_string());
        std::fs::create_dir_all(server_dir.clone()).expect("");
        let mut ssh = DeployMachine::new(&s, None, None);
        let fnm_export = "multiparty.csv";
        std::fs::remove_file(fnm_export).ok();
        let cmd = format!(
            "sqlite3 /root/.rg/{}/data_store.sqlite \\\"SELECT hex(party_info) FROM multiparty;\\\" > /root/.rg/{}/{}",
            net_str,
            net_str,
            fnm_export
        );
        info!(" backup cmd Running command: {}", cmd);
        ssh.exes("sudo apt install -y sqlite3", &output_handler).await.expect("");
        ssh.exes(cmd, &output_handler).await.expect("");
        tokio::time::sleep(Duration::from_secs(1)).await;
        let user = s.username.unwrap_or("root".to_string());
        let res = redgold_common_no_wasm::cmd::run_bash_async(
            format!(
                "scp {}@{}:~/.rg/{}/{} {}",
                user, s.host.clone(), net_str, fnm_export, fnm_export)
        ).await.expect("");
        println!("Backup result: {:?}", res);
        let contents = std::fs::read_to_string(fnm_export).expect("");
        std::fs::remove_file(fnm_export).ok();
        std::fs::write(server_dir.join(fnm_export), contents).expect("");
    }
}

pub(crate) async fn restore_multiparty_share(p0: NodeConfig, server: ServerOldFormat) -> RgResult<()> {
    let net_str = p0.network.to_std_string();

    let latest = get_backup_latest_path(p0).await?;
    if latest.is_none() {
        return Ok(());
    }
    let latest = latest.expect("latest");
    let latest = latest.join(server.index.to_string());
    let mp_csv = latest.join("multiparty.csv");

    let mut ssh = DeployMachine::new(&server, None, None);
    let remote_mp_import_path = format!("/root/.rg/{}/multiparty-import.csv", net_str);
    let local_backup_path = mp_csv.to_str().expect("").to_string();
    println!("Copying {} to {}", local_backup_path.clone(), remote_mp_import_path);

    let contents = tokio::fs::read_to_string(&local_backup_path)
        .await
        .error_info("Failed to read multiparty csv")
        .add(local_backup_path)?;

    ssh.copy(&contents, remote_mp_import_path).await.expect("");

    // This was the original command used for making the csv export
    // let cmd = format!(
    //     "sqlite3 ~/.rg/{}/data_store.sqlite \"SELECT \
    //     room_id, keygen_time, hex(keygen_public_key), hex(host_public_key), self_initiated, \
    //     hex(local_share), hex(initiate_keygen) FROM multiparty;\" > ~/.rg/{}/{}",
    //     net_str,
    //     net_str,
    //     fnm_export
    // );
    ssh.exes("sudo apt install -y sqlite3", &None).await.expect("");

    // TODO: Need some kind of hex conversion function here, this import statement is wrong,
    // for now rely on reading it automatically from the node.
    // Now we want to use sqlite to import the csv file at remote_mp_import_path
    // // Import the CSV file into the SQLite database
    // let cmd = format!(
    //     "sqlite3 ~/.rg/{}/data_store.sqlite \".mode csv\" \".import '{}' multiparty\"",
    //     net_str,
    //     remote_mp_import_path
    // );
    // ssh.exes(&cmd, &None).await.expect("Failed to import multiparty CSV");

    Ok(())
}

async fn get_backup_latest_path(p0: NodeConfig) -> RgResult<Option<PathBuf>> {
    let secure_or = p0.secure_or().by_env(p0.network);
    let bk = secure_or.backups();

    // List bk directory and select the latest

    // Read the directory entries
    let mut entries = tokio::fs::read_dir(bk).await.error_info("FS read error")?;

    // Collect the entries into a vector of paths
    let mut paths = Vec::new();
    while let Some(entry) = entries.next_entry().await.error_info("Missing dir entry")? {
        paths.push(entry.path());
    }
    paths.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    let latest = paths.last().cloned();
    Ok(latest)
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedMultiparty {
    room_id: String,
    keygen_time: i64,
    keygen_public_key: PublicKey,
    host_public_key: PublicKey,
    self_initiated: bool,
    local_share: String,
    initiate_keygen: InitiateMultipartyKeygenRequest,
}

pub async fn check_updated_multiparty_csv(r: &Relay) -> RgResult<()> {
    let env = r.node_config.env_data_folder();
    if !env.multiparty_import().exists() {
        return Ok(())
    }
    let raw = env.multiparty_import_str().await?;
    for row in parse_mp_csv(raw)? {
        r.ds.multiparty_store.add_keygen(
            &row
        ).await?;
    };
    tokio::fs::remove_file(env.multiparty_import()).await.error_info("Failed to remove multiparty import")?;
    Ok(())
}

pub fn parse_mp_csv(contents: String) -> RgResult<Vec<PartyInfo>> {
    let mut res = vec![];

    for e in contents.split("\n") {
        if e.trim().is_empty() {
            continue;
        }
        res.push(PartyInfo::from_hex(e)?);
    }
    Ok(res)
}

#[ignore]
#[tokio::test]
pub async fn debug_fix_server() {
    let r = Relay::dev_default().await;
    let sdf = r.node_config.clone().secure_data_folder.expect("works");
    let servers = sdf.all().servers().expect("servers");
    let s = servers.iter().filter(|s| s.index == 4).next().expect("server 4");
    restore_multiparty_share(r.node_config.clone(), s.clone()).await.expect("");
}

#[ignore]
#[tokio::test]
pub async fn manual_parse_test() {
    let r = Relay::dev_default().await;

    let latest = get_backup_latest_path(r.node_config.clone()).await.expect("latest").expect("latest");
    let mp_csv = latest.join("4");
    let mp_csv = mp_csv.join("multiparty.csv");

    println!("Reading multiparty csv: {:?}", mp_csv);
    let raw = tokio::fs::read_to_string(mp_csv).await.expect("read mp csv");

    let result = parse_mp_csv(raw);
    for row in result.expect("parsed") {
        println!("Parsed row: {:?}", row);
    }
}