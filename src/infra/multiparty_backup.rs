use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use redgold_schema::{ErrorInfoContext, from_hex, ProtoSerde, RgResult, SafeOption};
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::servers::Server;
use redgold_schema::structs::{InitiateMultipartyKeygenRequest, NetworkEnvironment, PublicKey};
use crate::core::relay::Relay;
use crate::infra::deploy::DeployMachine;
use crate::node_config::NodeConfig;
use crate::util;

pub(crate) async fn backup_multiparty_local_shares(p0: NodeConfig, p1: Vec<Server>) {

    let net_str = p0.network.to_std_string();
    let time = util::current_time_unix();
    let secure_or = p0.secure_or().by_env(p0.network);
    let bk = secure_or.backups();
    let time_back = bk.join(time.to_string());

    for s in p1 {
        let server_dir = time_back.join(s.index.to_string());
        std::fs::create_dir_all(server_dir.clone()).expect("");
        let mut ssh = DeployMachine::new(&s, None, None);
        let fnm_export = "multiparty.csv";
        std::fs::remove_file(fnm_export).ok();
        let cmd = format!(
            "sqlite3 ~/.rg/{}/data_store.sqlite \"SELECT \
            room_id, keygen_time, hex(keygen_public_key), hex(host_public_key), self_initiated, \
            hex(local_share), hex(initiate_keygen) FROM multiparty;\" > ~/.rg/{}/{}",
            net_str,
            net_str,
            fnm_export
        );
        ssh.exes("sudo apt install -y sqlite3", &None).await.expect("");
        ssh.exes(cmd, &None).await.expect("");
        let user = s.username.unwrap_or("root".to_string());
        let res = util::cmd::run_bash_async(
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

pub(crate) async fn restore_multiparty_share(p0: NodeConfig, server: Server) -> RgResult<()> {
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
            row.local_share,
            row.room_id,
            row.initiate_keygen,
            row.self_initiated,
            Some(row.keygen_time)
        ).await?;
    };
    tokio::fs::remove_file(env.multiparty_import()).await.error_info("Failed to remove multiparty import")?;
    Ok(())
}

pub fn parse_mp_csv(contents: String) -> RgResult<Vec<ParsedMultiparty>> {

    // This was the original command used for making the csv export
    // let cmd = format!(
    //     "sqlite3 ~/.rg/{}/data_store.sqlite \"SELECT \
    //     room_id, keygen_time, hex(keygen_public_key), hex(host_public_key), self_initiated, \
    //     hex(local_share), hex(initiate_keygen) FROM multiparty;\" > ~/.rg/{}/{}",
    //     net_str,
    //     net_str,
    //     fnm_export
    // );
    let mut res = vec![];

    for e in contents.split("\n") {
        if e.trim().is_empty() {
            continue;
        }
        let mut parts = e.split("|");
        let room_id = parts.next().ok_msg("Missing room_id")?;
        let keygen_time = parts.next().ok_msg("Missing keygen_time")?.trim();
        let keygen_public_key = parts.next().ok_msg("Missing keygen_public_key")?;
        let host_public_key = parts.next().ok_msg("Missing host_public_key")?;
        let self_initiated = parts.next().ok_msg("Missing self_initiated")?;
        let self_initiated = self_initiated == "1";
        let local_share = parts.next().ok_msg("Missing local_share")?;
        let initiate_keygen = parts.next().ok_msg("Missing initiate_keygen")?;
        let mp = ParsedMultiparty {
            room_id: room_id.to_string(),
            keygen_time: keygen_time.parse::<i64>().error_info("Bad keygen_time")?,
            keygen_public_key: PublicKey::from_hex(keygen_public_key.to_string())?,
            host_public_key: PublicKey::from_hex(host_public_key.to_string())?,
            self_initiated,
            local_share: String::from_utf8(from_hex(local_share.to_string())?).error_info("Bad local_share")?,
            initiate_keygen: InitiateMultipartyKeygenRequest::proto_deserialize_hex(initiate_keygen.to_string())?,
        };
        res.push(mp);

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