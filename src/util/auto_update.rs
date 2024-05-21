use crate::node_config::NodeConfig;
use crate::schema::structs::NetworkEnvironment;
use redgold_schema::util::cmd::run_cmd;
// use crate::util::init_logger;
use redgold_schema::util::lang_util::remove_whitespace;
use log::{error, info};
use std::time::Duration;
use reqwest::ClientBuilder;
use tokio::time;
use redgold_schema::ErrorInfoContext;
use redgold_schema::structs::ErrorInfo;

const S3_PREFIX_URL: &str = "https://redgold-public.s3.us-west-1.amazonaws.com/release/";
// detect OS.
//
// enum ReleaseTarget {
//     linux,
//     mac,
//     Win
// }

fn get_s3_sha256_path(network_type: NetworkEnvironment) -> String {
    format!(
        "{}{}/redgold_linux_sha256_checksum",
        S3_PREFIX_URL,
        network_type.to_std_string()
    )
}

fn get_s3_linux_binary_path(network_type: NetworkEnvironment) -> String {
    format!(
        "{}{}/redgold_linux",
        S3_PREFIX_URL,
        network_type.to_std_string()
    )
}

fn get_s3_mac_binary_path(network_type: NetworkEnvironment) -> String {
    format!(
        "{}{}/redgold_mac",
        S3_PREFIX_URL,
        network_type.to_std_string()
    )
}

// wget https://redgold-public.s3.us-west-1.amazonaws.com/release/testnet-latest/redgold_linux -O redgold_linux
pub async fn pull_sha256_hash(
    network_type: NetworkEnvironment,
) -> Result<String, ErrorInfo> {
    let client = reqwest::Client::new();
    let res = client
        .get(get_s3_sha256_path(network_type))
        .send()
        .await.error_info("Send request failure")?
        .text()
        .await.error_info("decoding failure of text")?;
    Ok(res)
}
pub async fn get_s3_sha256_release_hash(
    network_type: NetworkEnvironment, timeout: Option<Duration>
) -> Result<String, ErrorInfo> {
    let client = ClientBuilder::new().timeout(timeout.unwrap_or(Duration::from_secs(2))).build()
        .error_info("Client build failure")?;
    let res = client
        .get(get_s3_sha256_path(network_type))
        .send()
        .await.error_info("Send failure")?
        .text()
        .await.error_info("text decoding failure")?;
    let res = res.replace("\n", "").trim().to_string();
    Ok(res)
}

// Move this into a trait
pub async fn get_s3_sha256_release_hash_short_id(
    network_type: NetworkEnvironment, timeout: Option<Duration>
) -> Result<String, ErrorInfo> {
    get_s3_sha256_release_hash(network_type, timeout).await.map(|s| {
        let len = s.len();
        let start = len - 9;
        s[start..len].to_string()
    })
}

#[ignore]
#[tokio::test]
async fn checksum_poll() {
    let hash = pull_sha256_hash(NetworkEnvironment::Dev).await.expect("hash missing");
    println!("hash: {}", hash);

}

// TODO: This should really be coming from other peers as well for
// authenticating S3 is accurately reflecting a new update.
pub async fn poll_update(
    _network_type: NetworkEnvironment,
    current_hash: String,
    duration: Duration
) {
    let mut interval = time::interval(duration);
    loop {
        interval.tick().await;
        let result = pull_sha256_hash(_network_type).await;
        match result {
            Ok(s) => {
                if remove_whitespace(&*s) != remove_whitespace(&*current_hash) {
                    info!("Found new sha256 hash current: {} new: {}", current_hash, s);
                    // info!("Downloading new node binary");

                    if std::env::var("REDGOLD_DOCKER").is_ok() {
                        // TODO: Start a new docker image that restarts this image to preserve container name
                        // Re-use the docker compose script.
                        // info!("Attemping to pull new docker image before restart");
                        // let out1 = run_cmd(
                        //     "docker".to_string(),
                        //     vec![
                        //         "pull".to_string(),
                        //         format!("redgoldio/redgold:{}", _network_type.to_std_string())
                        //     ],
                        // );
                        // info!("Download pull output: {} {}", out1.0, out1.1);
                    } else {
                        let path_exec = std::env::current_exe().expect("Can't find the current exe");
                        let exec_name = path_exec.file_name().expect("filename access failure").to_str()
                            .expect("Filename missing").to_string();
                        if exec_name.contains("linux") {
                            let out1 = run_cmd(
                                "wget",
                                vec![
                                    &*get_s3_linux_binary_path(_network_type),
                                    "-O",
                                    "redgold_linux_updated",
                                ],
                            );

                            let out2 = run_cmd("chmod", vec!["+x", "/root/redgold_linux_updated"]);
                            info!("Download output: {} {}", out1.0, out1.1);
                            info!("Chmod output: {} {}", out2.0, out2.1);

                        } else if exec_name.contains("mac") {
                            let out1 = run_cmd(
                                "wget",
                                vec![
                                    &*get_s3_mac_binary_path(_network_type),
                                    "-O",
                                    "redgold_mac_updated",
                                ],
                            );
                            let out2 = run_cmd("chmod", vec!["+x", "/root/redgold_linux_updated"]);
                            info!("Download output: {} {}", out1.0, out1.1);
                            info!("Chmod output: {} {}", out2.0, out2.1);
                        }
                    }
                    // TODO: Do this gracefully by updating node state? Or otherwise
                    // stopping acceptance of new transactions.
                    // maybe use a mpmc channel for this? for state updates with each internal
                    // class carrying state info?

                    // info!("Shutting down for update to new version");
                    // std::process::exit(0);
                }
            }
            Err(e) => {
                use redgold_schema::helpers::easy_json::json_or;
                error!("Error querying s3 for updated checksum check, {}", json_or(&e))
            }
        }
    }
}
// use std::env;
//
// println!("{}", env::consts::OS); // Prints the current OS.

pub fn auto_update_enabled(network_env: NetworkEnvironment, disabled: bool) -> bool {
    let disable_auto_update_types: Vec<NetworkEnvironment> =
        vec![NetworkEnvironment::Debug, NetworkEnvironment::Local];
    !disable_auto_update_types.contains(&network_env) || !disabled
}

#[test]
fn verify_auto_update_start() {
    assert!(auto_update_enabled(NetworkEnvironment::Dev, false));
}

// TODO: This should really poll peers to see if they have a newly updated version.
pub async fn from_node_config(node_config: NodeConfig) {
    if let Some(hash) = node_config.executable_checksum {
        if auto_update_enabled(node_config.network, node_config.disable_auto_update)
        {
            info!(
                "auto update disabled but would be Starting auto update process from CURRENT_EXE_SHA256={}",
                hash
            );
            // poll_update(
            //     node_config.network_type,
            //     hash,
            //     node_config.auto_update_poll_interval
            // )
            // .await
        } else {
            info!("Auto update disabled for network type: {} with config disable_auto_update {:?}",
                node_config.network.to_std_string(), node_config.disable_auto_update);
        }
    }
}

#[tokio::test]
async fn try_query() {
    // init_logger();
                        // poll_update(
                        //     NetworkEnvironment::Test,
                        //     "Asdf".to_string(),
                        //     time::Duration::from_secs(3),
                        // )
                        // .await
    let result = pull_sha256_hash(NetworkEnvironment::Test).await;
    println!("{:?}", result);
}

#[test]
fn debug() {
    use std::env;

    println!("{}", env::consts::OS); // Prints the current OS.
}
