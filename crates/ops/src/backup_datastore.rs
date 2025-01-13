use std::time::Duration;
use tracing::info;
use redgold_common_no_wasm::output_handlers::log_handler;
use redgold_common_no_wasm::ssh_like::DeployMachine;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::servers::ServerOldFormat;
use redgold_schema::util;
use redgold_schema::util::times::{current_time_millis, ToTimeString};

pub async fn restore_datastore_servers(p0: NodeConfig, p1: Vec<ServerOldFormat>) {

    let net_str = p0.network.to_std_string();
    let secure_or = p0.secure_or().by_env(p0.network);
    let bk = secure_or.backups_ds();

    // List dir on backups path:
    let mut dirs = vec![];
    for dir in std::fs::read_dir(bk.clone()).expect("read dir") {
        let path = dir.expect("dir").path();
        dirs.push(path);
    }
    dirs.sort();
    if let Some(d) = dirs.last() {
        let handler = log_handler().1;
        for s in p1 {
            let server_dir = d.join(s.index.to_string());
            let ds_dir = server_dir.join("data_store.sqlite");
            let ds_str = ds_dir.to_str().expect("").to_string();
            info!("Restoring data store for server: {} with path {}", s.index, ds_str);

            let mut ssh = DeployMachine::new(&s, None, handler.clone());
            let user = s.username.unwrap_or("root".to_string());
            let backup_cmd = format!(
                "scp {} {}@{}:~/.rg/{}/{}",
                ds_str, user, s.host.clone(), net_str, "data_store.sqlite"
            );
            info!(" Restore cmd Running command: {}", backup_cmd);
            let res = redgold_common_no_wasm::cmd::run_bash_async(
                backup_cmd
            ).await.expect("");
            info!("Restore result: {:?}", res);
            info!("Removing SHM and WAL");
            ssh.exes(r"rm -f ~/.rg/{net_str}/data_store.sqlite-shm", &handler).await.expect("");
            ssh.exes(r"rm -f ~/.rg/{net_str}/data_store.sqlite-wal", &handler).await.expect("");
        }
    }

}
pub async fn backup_datastore_servers(p0: NodeConfig, p1: Vec<ServerOldFormat>) {

    let net_str = p0.network.to_std_string();
    let time_ms = current_time_millis();
    let secure_or = p0.secure_or().by_env(p0.network);
    let bk = secure_or.backups_ds();
    let time = time_ms.to_time_string_shorter_underscores();
    let time_back = bk.join(time.to_string());
    let (default_fun, output_handler) = log_handler();

    for s in p1 {
        let server_dir = time_back.join(s.index.to_string());
        std::fs::create_dir_all(server_dir.clone()).expect("");
        let mut ssh = DeployMachine::new(&s, None, None);
        let user = s.username.unwrap_or("root".to_string());
        let backup_cmd = format!(
            "scp {}@{}:~/.rg/{}/{} {}",
            user, s.host.clone(), net_str, "data_store.sqlite", server_dir.join("data_store.sqlite").to_str().unwrap());
        info!(" backup cmd Running command: {}", backup_cmd);
        let res = redgold_common_no_wasm::cmd::run_bash_async(
            backup_cmd
        ).await.expect("");
        info!("Backup result: {:?}", res);
    }
}
