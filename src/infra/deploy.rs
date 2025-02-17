use flume::Sender;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

use itertools::Itertools;
use redgold_common_no_wasm::cmd::run_command_os;
use redgold_common_no_wasm::data_folder_read_ext::EnvFolderReadExt;
use redgold_common_no_wasm::ssh_like::{DeployMachine, LocalSSHLike, SSHOrCommandLike, SSHProcessInvoke};
use redgold_common_no_wasm::tx_new::TransactionBuilderSupport;
use redgold_keys::address_support::AddressSupport;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::util::mnemonic_support::MnemonicSupport;
use redgold_schema::constants::default_node_internal_derivation_path;
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::keys::words_pass::WordsPass;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::servers::ServerOldFormat;
use redgold_schema::structs::{ErrorInfo, NetworkEnvironment, PeerId, PeerMetadata, SupportedCurrency, Transaction, TrustRatingLabel};
use redgold_schema::tx::tx_builder::TransactionBuilder;
use redgold_schema::{structs, ErrorInfoContext, RgResult, SafeOption};
use std::io::prelude::*;
use std::sync::Arc;
use eframe::egui::TextBuffer;
use crate::hardware::trezor;
use crate::hardware::trezor::trezor_bitcoin_standard_path;
use crate::resources::Resources;
use crate::util::cli::arg_parse_config::{get_default_data_top_folder, ArgTranslate};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::conf::rg_args::Deploy;
use redgold_schema::conf::server_config::{Deployment, NodeInstance, ServerData};
use redgold_schema::config_data::ConfigData;
use redgold_schema::data_folder::DataFolder;
use crate::infra::multiparty_backup::restore_multiparty_share;

#[ignore]
#[tokio::test]
async fn debug_ssh_invoke() {
    let host = "hostnoc".to_string();
    let ssh = SSHProcessInvoke {
        user: None,
        identity_path: None,
        host: host.clone(),
        strict_host_key_checking: false,
        output_handler: None,
        jump_host: None,
    };
    let result = ssh.execute("ls", None).await.expect("ssh");
    println!("Result: {}", result);

    let s = ServerOldFormat {
        name: "".to_string(),
        host,
        index: 0,
        peer_id_index: 0,
        network_environment: "".to_string(),
        username: None,
        ipv4: None,
        node_name: None,
        external_host: None,
        reward_address: None,
        jump_host: None,
        party_config: None,
    };

    let mut dm = DeployMachine::new(&s, None, None);
    // dm.verify().expect("verify");
    let res = dm.exes("ls ~/.rg", &None).await.expect("ls");
    println!("Result2: {}", res);
}

/**
Updates to this cannot be explicitly watched through docker watchtower for automatic updates
They must be manually deployed.
 */
pub async fn deploy_redgold<T: SSHOrCommandLike>(
    mut ssh: DeployMachine<T>,
    network: NetworkEnvironment,
    is_genesis: bool,
    additional_env: Option<HashMap<String, String>>,
    purge_data: bool,
    words: Option<String>,
    peer_id_hex: Option<String>,
    start_node: bool,
    alias: Option<String>,
    ser_pid_tx: Option<String>,
    p: &Option<Sender<String>>,
    disable_system: bool,
    config_data: Arc<ConfigData>,
    dpl_tuple: Option<(Deployment, ServerData, NodeInstance)>,
    nc: &Box<NodeConfig>
) -> Result<(), ErrorInfo> {
    let is_local = dpl_tuple.as_ref().and_then(|(_, s, n)| {
        s.is_localhost
    }).unwrap_or(false);
    let data_folder = if ssh.server.host.is_empty() {
        // home directory
        let mut home = get_default_data_top_folder();
        if let Some((_, _, ni)) = dpl_tuple.as_ref() {
            if ni.use_id_ds_prefix.unwrap_or(false) {
                home = home.join("id_index");
                home = home.join(ni.index.unwrap_or(0).to_string());
            }
        }
        home.to_str().unwrap().to_string()
    } else {
        if let Some(u) = ssh.server.username.as_ref() {
            format!("/home/{}/.rg", u)
        } else {
            "/root/.rg".to_string()
        }
    };

    ssh.verify().await?;

    let _host = ssh.server.host.clone();
    ssh.exes("docker system prune -a -f", p).await?;

    if !disable_system {
        ssh.exes("sudo apt update", p).await?;
        // Issue here with command hanging.
        // E: dpkg was interrupted, you must manually run 'sudo dpkg --configure -a' to correct the problem.
        // ssh.exes("sudo apt upgrade -y", p).await?;
        ssh.exes("apt install -y ufw", p).await?;
        ssh.exes("sudo ufw allow ssh", p).await?;
        ssh.exes("sudo ufw allow in on tailscale0", p).await?;
        ssh.exes("echo 'y' | sudo ufw enable", p).await?;

        let compose = ssh.exes("docker-compose", p).await?;
        ssh.exes("sudo apt-get install docker-compose-plugin", p).await?;
        if !(compose.contains("applications")) {
            ssh.exes("curl -fsSL https://get.docker.com -o get-docker.sh; sh ./get-docker.sh", p).await?;
            ssh.exes("sudo apt install -y docker-compose", p).await?;
        }
    }
    let r = Resources::default();

    let path = format!("{}/{}", data_folder, network.to_std_string());
    let all_path = format!("{}/{}",data_folder, NetworkEnvironment::All.to_std_string());
     let maybe_main_path = if network == NetworkEnvironment::Main {
         path.clone()
     } else {
         all_path.clone()
     };

    ssh.exes(format!("mkdir -p {}", path), p).await?;;
    ssh.exes(format!("mkdir -p {}", all_path), p).await?;;
     // Copy mnemonic / peer_id
     if let Some(words) = words.clone() {
         if network != NetworkEnvironment::Main {
             let env_remote = format!("{}/mnemonic", path);
             ssh.exes(format!("rm {}", env_remote), p).await?;
         }
         let remote = format!("{}/mnemonic", maybe_main_path);
         ssh.copy_p(words, remote, p).await?;
     }
     if let Some(peer_id_hex) = peer_id_hex {
         let remote = format!("{}/peer_id", path);
         ssh.copy_p(peer_id_hex, remote, p).await?;
     }
     if let Some(tx) = ser_pid_tx {
         let remote = format!("{}/peer_tx", path);
         ssh.copy_p(tx, remote, p).await?;
     }


    // TODO: Investigate issue with tmpfile, not working
    // // let mut tmpfile: File = tempfile::tempfile().unwrap();
    // // write!(tmpfile, "{}", r.redgold_docker_compose).unwrap();
    // TODO: Also wget from github directly depending on security concerns -- not verified from checksum hash
    // Only should be done to override if the given exe is outdated.
    ssh.exes(format!("mkdir -p {}", path), p).await?;
    let compose_str = r.redgold_docker_compose;
    let port = network.default_port_offset();

    let all_open_ports = vec![-1, 0, 1, 4, 5, 6].iter().map(|p| ((port as i64) + p) as u16)
        .collect_vec();

    if is_local {

        let hostname = run_command_os("hostname".to_string()).await?.0;
        let mut replace_str = r#"ports:
      {{PORTS}}
    deploy:
      placement:
        constraints:
          - "node.hostname=={{your-node-name}}"#.to_string();
        replace_str = replace_str.replace("{{your-node-name}}", &hostname);
        let ports = all_open_ports.iter().map(|p| format!("- '{}':'{}'", p, p)).join("\n    ");
        replace_str = replace_str.replace("{{PORTS}}", &ports);
        compose_str.replace("network_mode: host", &*replace_str);
    }

    ssh.copy_p(compose_str, format!("{}/redgold-only.yml", path), p).await?;

    let mut env = additional_env.unwrap_or(Default::default());
    env.insert("REDGOLD_NETWORK".to_string(), network.to_std_string());
    // env.insert("REDGOLD_GENESIS".to_string(), is_genesis.to_string());
    // env.insert("REDGOLD_METRICS_PORT".to_string(), format!("{}", port - 1));
    // env.insert("REDGOLD_P2P_PORT".to_string(), format!("{}", port));
    // env.insert("REDGOLD_PUBLIC_PORT".to_string(), format!("{}", port + 1));
    // env.insert("REDGOLD_CONTROL_PORT".to_string(), format!("{}", port + 2));
    env.insert("RUST_BACKTRACE".to_string(), "full".to_string());
    env.insert("RUST_LOG".to_string(), "debug".to_string());
    // env.insert("REDGOLD_SERVER_INDEX".to_string(), ssh.server.index.to_string());
    // env.insert("REDGOLD_SERVER_PEER_INDEX".to_string(), ssh.server.peer_id_index.to_string());
    // env.insert("REDGOLD_SERVER_NODE_NAME".to_string(), ssh.server.node_name.clone().unwrap_or("anon".to_string()));
    // TODO: Change to _main

     if let Some(a) = alias {
         // env.insert("REDGOLD_ALIAS".to_string(), a);
     }
    // TODO: Inherit from node_config?
    let copy_env = vec![
        "AWS_ACCESS_KEY_ID",
        "AWS_SECRET_ACCESS_KEY",
        "ETHERSCAN_API_KEY",
        "RECAPTCHA_SECRET",
        "REDGOLD_MAIN_DEVELOPMENT_MODE",
        "REDGOLD_DEVELOPMENT_MODE",
        "REDGOLD_TO_EMAIL",
        "REDGOLD_FROM_EMAIL",
    ];
    for e in copy_env {
        for i in std::env::var(e).ok() {
            env.insert(e.to_string(), i);
        }
    }

    // TODO: Env vars should all be migrated here, secure should be erased
    // Prepare config data
    let mut config = (*config_data).clone();
    // info!("Config on deploy remove this: {:?}", config);
    config.home = None;
    config.config = None;
    config.secure = None;
    config.local = None;
    config.network = Some(network.to_std_string());
    // TODO: Support other users here.
    config.data = Some(data_folder.to_string());
    let node = config.node.get_or_insert(Default::default());
    node.words = words.clone();
    node.server_index = Some(ssh.server.index);
    node.peer_id_index = Some(ssh.server.peer_id_index);
    node.name = Some(ssh.server.node_name.clone().unwrap_or("anon".to_string()));
    let debug = config.debug.get_or_insert(Default::default());
    debug.genesis = Some(is_genesis);
    debug.developer = Some(env.get("REDGOLD_MAIN_DEVELOPMENT_MODE").is_some());
    debug.develop = Some(env.get("REDGOLD_DEVELOPMENT_MODE").is_some());
    let keys = config.keys.get_or_insert(Default::default());
    keys.aws_access = env.get("AWS_ACCESS_KEY_ID").cloned();
    keys.aws_secret = env.get("AWS_SECRET_ACCESS_KEY").cloned();
    keys.etherscan = env.get("ETHERSCAN_API_KEY").cloned();
    keys.recaptcha = env.get("RECAPTCHA_SECRET").cloned();
    let email = config.email.get_or_insert(Default::default());
    email.to = env.get("REDGOLD_TO_EMAIL").cloned();
    email.from = env.get("REDGOLD_FROM_EMAIL").cloned();
    let party = config.party.get_or_insert(Default::default());
    let external = config.external.get_or_insert(Default::default());
    party.party_config = ssh.server.party_config.clone();

    let rpcs = external.rpcs.get_or_insert(Default::default());
    if let Some((t,t2, t3)) = dpl_tuple.as_ref() {
        if let Some(overrides) = t3.rpc_overrides.as_ref() {
            let curs = overrides.iter().map(|r| r.currency).collect_vec();
            rpcs.retain(|k| !curs.contains(&k.currency));
            for r in overrides {
                rpcs.push(r.clone())
            }
        }
    }

    if ssh.server.index == 0 && nc.development_mode() {
        if nc.development_mode_main() {
            party.enable = Some(true);
            debug.enable_live_e2e = Some(true);
            // env.insert("REDGOLD_ENABLE_PARTY_MODE".to_string(), "true".to_string());
            // env.insert("REDGOLD_LIVE_E2E_ENABLED".to_string(), "true".to_string());
        };
        debug.grafana_writer = Some(true);
        // env.insert("REDGOLD_GRAFANA_PUBLIC_WRITER".to_string(), "true".to_string());
    }
    if nc.development_mode_main() {
        // REDGOLD_MAIN_DEVELOPMENT_MODE
        debug.developer = Some(true);
        // env.insert("REDGOLD_MAIN_DEVELOPMENT_MODE".to_string(), "true".to_string());
        // env.insert("REDGOLD_S3_BACKUP_BUCKET".to_string(), "redgold-backups".to_string());
        external.s3_backup_bucket = Some("redgold-backups".to_string());
    }

    let cloned = config.clone();
    let mut config2 = cloned.clone();
    let toml_config = toml::to_string(&cloned).error_info("toml config")?;
    // info!("Copying toml config {}", toml_config.clone());
    ssh.copy_p(toml_config, format!("{}/config.toml", path), p).await?;

    if !disable_system {
        // TODO: Lol not this
        let port_range: Vec<i64> = vec![-1, 0, 1, 4, 5, 6];
        for port_i in port_range {
            let port_o = (port as i64) + port_i;
            ssh.exes(format!("sudo ufw allow proto tcp from any to any port {}", port_o), p).await?;
            ssh.exes(format!("sudo ufw allow proto udp from any to any port {}", port_o), p).await?;
        }
    }

    let env_contents = env.iter().map(|(k, v)| {
        format!("{}={}", k, format!("{}", v))
    }).join("\n");
    ssh.copy_p(env_contents.clone(), format!("{}/var.env", path), p).await?;
    ssh.copy_p(env_contents, format!("{}/.env", path), p).await?;

    sleep(Duration::from_secs(4));

    ssh.exes(format!("cd {}; docker-compose -f redgold-only.yml down", path), p).await?;

    if purge_data {
        println!("Purging data");
        ssh.exes(format!("rm -rf {}/{}", path, "data_store.sqlite"), p).await?;
    }

    if !disable_system {
        ssh.exes("sudo ufw reload", p).await?;
    }
    ssh.exes(format!("cd {}; docker-compose -f redgold-only.yml pull", path), p).await?;
    if start_node {

        if is_local {
            // swarm init before hand if system level
            // .
            ssh.exes(format!("cd {}; docker stack deploy -c redgold-only.yml redgold-{}", path, network.to_std_string()), p).await?;
        } else {
            ssh.exes(format!("cd {}; docker-compose -f redgold-only.yml up -d", path), p).await?;
        }
        if is_genesis {
            // After starting node for the first time, mark the environment file as not genesis
            // for the next time.
            // env.remove("REDGOLD_GENESIS");
            // // TODO: Move this to an Deploy class with an SSHLike trait as an inner.
            // // so it's a repeated function.
            // let env_contents = env.iter().map(|(k, v)| {
            //     format!("{}={}", k, format!("{}", v))
            // }).join("\n");
            // ssh.copy_p(env_contents.clone(), format!("{}/var.env", path), p).await?;
            // ssh.copy_p(env_contents, format!("{}/.env", path), p).await?;
            config2.debug.get_or_insert(Default::default()).genesis = Some(false);
            let toml_config = toml::to_string(&config2).error_info("toml config")?;
            ssh.copy_p(toml_config, format!("{}/config.toml", path), p).await?;
        }
    }

    // monero wallet rpc
    let override_rpc = dpl_tuple.as_ref().and_then(|d| d.2.rpc_overrides.as_ref())
        .and_then(|r| r.iter()
            .filter(|x| {
                x.currency == SupportedCurrency::Monero &&
                    x.network == network.to_std_string() &&
                    x.wallet_only == Some(true) &&
                    x.ws_only == Some(false) &&
                    x.authentication.is_some()
            }).next()).cloned();

    let default_rpc = nc.rpc_url(SupportedCurrency::Monero)
        .iter()
        .filter(|x| {
            x.currency == SupportedCurrency::Monero &&
                x.network == network.to_std_string() &&
                x.wallet_only == Some(true) &&
                x.ws_only == Some(false) &&
                x.authentication.is_some()
        }).next().cloned();

    let wallet_rpc = override_rpc.or(default_rpc);

    // monero wallet rpc
    let override_rpc = dpl_tuple.as_ref().and_then(|d| d.2.rpc_overrides.as_ref())
        .and_then(|r| r.iter()
            .filter(|x| {
                x.currency == SupportedCurrency::Monero &&
                    x.network == network.to_std_string() &&
                    x.ws_only == Some(false) &&
                    x.wallet_only == Some(false)
            }).next()).cloned();

    let default_rpc = nc.rpc_url(SupportedCurrency::Monero)
        .iter()
        .filter(|x| {
            x.currency == SupportedCurrency::Monero &&
                x.network == network.to_std_string() &&
                x.ws_only == Some(false) &&
                x.wallet_only == Some(false)
        }).next().cloned();

    let daemon_rpc = override_rpc.or(default_rpc);

    if let (Some(wallet_rpc), Some(daemon_rpc)) = (wallet_rpc, daemon_rpc) {
        let mut compose_str = r.monero_rpc_wallet_compose;
        let container_name = format!("monerow-{}", network.to_std_string());
        compose_str = compose_str.replace("${CONTAINER_NAME:-monerotw}", container_name.as_str());
        let data_dir_path = format!("{}/monerow", path);
        compose_str = compose_str.replace("${WALLET_DATA_DIR:-/disk/monerotw}", data_dir_path.as_str());
        compose_str = compose_str.replace("${RPC_LOGIN:-username:password}", wallet_rpc.authentication.clone().unwrap().as_str());
        compose_str = compose_str.replace("${DAEMON_HOST:-http://127.0.0.1:28089}", daemon_rpc.url.as_str());
        compose_str = compose_str.replace("${WALLET_RPC_PORT:-28088}", wallet_rpc.url.as_str().split(":").last().unwrap());

        // if network.is_main() {
        //     compose_str = compose_str.replace("      - --testnet", "");
        //     compose_str = compose_str.replace("      - --disable-rpc-ban", "");
        // }
        let yaml_path = format!("{}/monero-wallet.yml", path);
        let wallet_exp_path = format!("{}/wallet.exp", path);

        ssh.copy_p(compose_str, yaml_path, p).await?;
        ssh.copy_p(r.monero_rpc_wallet_expect, wallet_exp_path, p).await?;

    }

    Ok(())
}

pub async fn deploy_ops_services(
    mut ssh: DeployMachine<SSHProcessInvoke>,
    _additional_env: Option<HashMap<String, String>>,
    remote_path_prefix: Option<String>,
    grafana_pass: Option<String>,
    purge_data: bool,
    p: &Option<Sender<String>>,
    skip_start: bool,
    node_exporter_template: Option<Vec<ServerOldFormat>>,
    skip_logs: bool,
    include_smtp: bool,
    allow_anon_read: bool
) -> Result<(), ErrorInfo> {

    let node_exporter_template = node_exporter_template
        .map(|n| n.iter().map(|s| format!("'{}:9100'", s.host.clone())).join(","));
    let remote_path = remote_path_prefix.unwrap_or("/root/.rg/all".to_string());
    ssh.verify().await?;
    //
    // let p = &Box::new(|s: String| {
    //     println!("Partial output: {}", s);
    //     Ok(())
    // });

    ssh.exes("docker ps", p).await?;
    ssh.copy(
        include_str!("../resources/infra/ops_services/services-all.yml"),
        format!("{}/services-all.yml", remote_path)
    ).await?;
    ssh.copy(
        include_str!("../resources/infra/ops_services/services-nologs.yml"),
        format!("{}/services-nologs.yml", remote_path)
    ).await?;

    ssh.copy(
        include_str!("../resources/infra/ops_services/filebeat.docker.yml"),
        format!("{}/filebeat.docker.yml", remote_path)
    ).await?;

//     match std::env::var("GRAFANA_CLOUD_USER") {
//         Ok(u) => {
//             promtheus_yml += &*format!("remote_write:
// - url: {}
//   basic_auth:
//     username: {}
//     password: {}",
//                                        u,
//                                        std::env::var("GRAFANA_CLOUD_URL").expect(""),
//                                        std::env::var("GRAFANA_CLOUD_API").expect("")
//             );
//         }
//         Err(_) => {}
//     }
    let prometheus_yml = include_str!("../resources/infra/ops_services/prometheus.yml").to_string();
    let replaced_prometheus_yml = match node_exporter_template {
        None => {
            prometheus_yml
        }
        Some(templ) => {
            prometheus_yml.replace("'localhost:9100'", format!("'localhost:9100', {}", templ).as_str())
        }
    };

    ssh.copy(
        replaced_prometheus_yml,
        format!("{}/prometheus.yml", remote_path)
    ).await?;
    ssh.copy(
        include_str!("../resources/infra/ops_services/prometheus-datasource.yaml"),
        format!("{}/prometheus-datasource.yaml", remote_path)
    ).await?;

    ssh.copy(
        grafana_pass.unwrap_or("debug".to_string()),
        format!("{}/grafana_password", remote_path)
    ).await?;

    ssh.exes(format!("rm -r {}/dashboards", remote_path), p).await?;
    ssh.exes(format!("mkdir {}/dashboards", remote_path), p).await?;

    let x = include_str!("../resources/infra/ops_services/dashboards/node-exporter-full_rev31.json");
    ssh.copy(
        x,
        format!("{}/dashboards/node-exporter.json", remote_path)
    ).await?;

    let x = include_str!("../resources/infra/ops_services/dashboards/redgold_rev0.json");
    ssh.copy(
        x,
        format!("{}/dashboards/redgold.json", remote_path)
    ).await?;

    // println!("Copying node exporter dashboard: {}", x);

    ssh.copy(
        include_str!("../resources/infra/ops_services/dashboards/dashboard_config.yaml"),
        format!("{}/dashboards/dashboard_config.yaml", remote_path)
    ).await?;

    let mut grafana_ini = include_str!("../resources/infra/ops_services/grafana/grafana.ini").to_string();
    if allow_anon_read {
        let anon_str =
            "[auth.anonymous]
enabled = true
org_role = Viewer
";
        grafana_ini = grafana_ini.replace("[auth.anonymous]", anon_str);
    }
    ssh.copy(
        grafana_ini,
        format!("{}/grafana.ini", remote_path)
    ).await?;

    // Environment
    let mut env = _additional_env.unwrap_or(Default::default());
    env.insert("GF_SECURITY_ADMIN_PASSWORD__FILE".to_string(), "/etc/grafana/grafana_secret".to_string());
    let copy_env = vec!["SMTP_HOST", "SMTP_USER", "SMTP_PASSWORD", "SMTP_FROM_ADDRESS", "SMTP_FROM_NAME"];
    for e in copy_env {
        for i in std::env::var(e).ok() {
            let ii = if !include_smtp && e == "SMTP_PASSWORD" {
                "".to_string()
            } else {
                i
            };
            env.insert(e.to_string(), ii);
        }
    }

    let env_contents = env.iter().map(|(k, v)| {
        format!("{}={}", k, format!("{}", v))
    }).join("\n");
    ssh.copy(env_contents.clone(), format!("{}/ops_var.env", remote_path)).await?;

    ssh.exes(format!("cd {}; docker-compose -f services-all.yml down", remote_path), p).await?;
    ssh.exes(format!("cd {}; docker-compose -f services-nologs.yml down", remote_path), p).await?;

    for s in vec!["grafana", "prometheus", "esdata"] {
        if purge_data {
            ssh.exes(format!("rm -r {}/data/{}", remote_path, s), p).await?;
        }
        ssh.exes(format!("mkdir -p {}/data/{}", remote_path, s), p).await?;
    };

    ssh.exes(format!("chmod -R 777 {}/data/esdata", remote_path), p).await?;

    let kibana_setup_path = format!("{}/kibana_setup.sh", remote_path);
    ssh.copy(
        include_str!("../resources/infra/ops_services/kibana_setup.sh"),
        kibana_setup_path.clone()
    ).await?;
    ssh.exes(format!("chmod +x {}", kibana_setup_path.clone()), p).await?;


    if !skip_start {
        if skip_logs {
            ssh.exes(format!("cd {}; docker-compose -f services-nologs.yml up -d", remote_path), p).await?;
        } else {
            ssh.exes(format!("cd {}; docker-compose -f services-all.yml up -d", remote_path), p).await?;
            // Wait for ES to come online
            tokio::time::sleep(Duration::from_secs(60)).await;

            ssh.exes(format!("{}", kibana_setup_path), p).await?;
        }
    }

    Ok(())
}


pub async fn derive_mnemonic_and_peer_id(
    node_config: &NodeConfig,
    mnemonic: String,
    peer_id_index: usize,
    cold: bool,
    passphrase: Option<String>,
    opt_peer_id: Option<String>,
    server_id_index: i64,
    servers: Vec<ServerOldFormat>,
    trust: Vec<TrustRatingLabel>,
    peer_id_tx: &mut HashMap<String, structs::Transaction>,
    net: &NetworkEnvironment,
    reward_address: Option<String>
)
    -> RgResult<(String, String)> {

    // TODO: Make peer id transaction here using details.
    let w = WordsPass::new(mnemonic, passphrase);
    let new = w.hash_derive_words(server_id_index.to_string())?;
    let server_mnemonic = new.words.clone();
    let account = (99 - peer_id_index) as u32;
    let mut pid_hex = "".to_string();
    let mut pubkey = None;
    if let Some(pid) = opt_peer_id {
        pid_hex = pid;
    } else {
        let pk = if cold {
            trezor::get_standard_public_key(
                account, None, 0, 0)?
        } else {
            let result = new.default_peer_id();
            result?.peer_id.expect("pid")
        };
        pubkey = Some(pk.clone());
        pid_hex = PeerId::from_pk(pk).hex();
    }
    if !peer_id_tx.contains_key(&pid_hex) {

        let pkey = pubkey.expect("k");
        let mut peer_data = PeerMetadata::default();
        peer_data.peer_id = Some(PeerId::from_pk(pkey.clone()));

        let mut pkmap = HashMap::default();
        pkmap.insert(server_id_index, new.default_public_key().expect("pk"));
        ServerOldFormat::peer_data(
            servers.clone(),
            &mut peer_data,
            peer_id_index as i64,
            pkmap,
            node_config.executable_checksum.clone().unwrap_or("".to_string()),
            net.clone(),
            reward_address.clone().and_then(|a| a.parse_address().ok()),
        );
        peer_data.labels = trust.clone();
        let mut tb = TransactionBuilder::new(&node_config);
        let address = pkey.address().expect("a");
        tb.with_output_peer_data(&address, peer_data, 0);
        tb.with_genesis_input(&address);
        let hash = tb.transaction.hash_or();
        let mut input = tb.transaction.inputs.last_mut().expect("");
        if cold {
            trezor::sign_input(
                &mut input, &pkey, trezor_bitcoin_standard_path(
                    account, None, 0, 0
                ), &hash
            ).await?;
        } else {
            let result = new.keypair_at(default_node_internal_derivation_path(1))?;
            tb.transaction.sign(&result)?;
        };
        peer_id_tx.insert(pid_hex.clone(), tb.transaction.clone());
    }
    Ok((server_mnemonic, pid_hex))
}


/// Allow offline (airgapped) generation of peer TX / node TX from servers manifest
pub async fn offline_generate_keys_servers(
    node_config: NodeConfig,
    servers: Vec<ServerOldFormat>,
    save_path: PathBuf,
    salt_mnemonic: String,
    passphrase: Option<String>
) -> RgResult<Vec<(String, String)>> {
    let mut pid_tx: HashMap<String, structs::Transaction> = HashMap::default();
    let mut res = vec![];
    for ss in &servers {
        let (words, peer_id_hex) = derive_mnemonic_and_peer_id(
            &node_config,
            salt_mnemonic.clone(),
            ss.peer_id_index as usize,
            false,
            passphrase.clone(),
            None,
            ss.index,
            servers.clone(),
            vec![],
            &mut pid_tx,
            &node_config.network,
            ss.reward_address.clone()
        ).await?;
        let peer_tx = pid_tx.get(&peer_id_hex).expect("").clone();
        let pd = peer_tx.peer_data()?;
        let nmd = pd.node_metadata.get(0).ok_msg("Missing node metadata")?;
        let pk = nmd.public_key.as_ref().ok_msg("Missing public key")?;
        let pub_hex = pk.hex();
        let peer_tx_ser = peer_tx.json_or();
        let save = save_path.clone();
        let server_index_path = save.join(format!("{}", ss.index));
        std::fs::create_dir_all(server_index_path.clone()).expect("");
        let peer_tx_path = server_index_path.join("peer_tx");
        let words_path = server_index_path.join("mnemonic");
        std::fs::write(peer_tx_path, peer_tx_ser).expect("");
        std::fs::write(words_path, words).expect("");
        res.push((peer_id_hex, pub_hex));
    }
    Ok(res)
}


pub async fn default_deploy(
    deploy: &mut Deploy,
    node_config: &NodeConfig,
    output_handler: Option<Sender<String>>,
    servers_opt: Option<Vec<ServerOldFormat>>,
    deployment: Option<Deployment>
) -> RgResult<()> {

    let deployment = deployment.map(|d| d.fill_params());

    // let primary_gen = std::env::var("REDGOLD_PRIMARY_GENESIS").is_ok();
    // if node_config.opts.development_mode {
    //     // Also set environment here to dev if not main
    //     deploy.skip_ops = true;
    // }
    let net = node_config.network;

    if net == NetworkEnvironment::Main {
        // TODO: Does this matter?
        // deploy.ask_pass = true;
    } else {
        deploy.words_and_id = true;
    }


    let sd = ArgTranslate::secure_data_path_buf().expect("");
    let sd = sd.join(".rg");
    let df = DataFolder::from_path(sd);
    let m = df.all().mnemonic().await.expect("");
    let passphrase = deploy.mixing_password.clone().or_else(|| {
        if deploy.ask_pass {
        let passphrase = rpassword::prompt_password("Enter passphrase for mnemonic: ").unwrap();
        let passphrase2 = rpassword::prompt_password("Re-enter passphrase for mnemonic: ").unwrap();
        if passphrase != passphrase2 {
            panic!("Passphrases do not match");
        }
        if passphrase.is_empty() {
            None
        } else {
            Some(passphrase)
        }
    } else {
        None
    }});
    // Ok heres what to do, in here we need to potentially invoke the HW signer for peer id
    // if we don't have one generated FOR THE ENVIRONMENT of interest.
    // So check to see if the peer id exists, if not, generate it according to hardware signer
    // ONLY IF mainnet do we use hardware signer?
    //WordsPass::new(m)

    let servers_original = if let Some(servers_preload) = servers_opt {
        servers_preload
    } else {
        let buf = df.all().servers_path();
        // println!("Reading servers file: {:?}", buf);
        ArgTranslate::read_servers_file(buf)?
    };

    // TODO: Filter servers by environment, also optionally pass them from the GUI?
    // println!("Setting up servers: {:?}", servers_original);
    // let mut gen = true;
    let purge = deploy.purge;
    let mut gen = deploy.genesis;
    // if primary_gen {
    //     gen = true;
    // }
    let mut hm = HashMap::new();
    hm.insert("RUST_BACKTRACE".to_string(), "1".to_string());

    let mut servers = servers_original.to_vec();
    if let Some(i) = deploy.server_index {
        let x = servers.iter().filter(|s| s.index == (i as i64)).next().expect("").clone();
        servers = vec![x]
    }
    if let Some(csv_filter) = &deploy.server_filter {
        let split = csv_filter.split(",").collect_vec();
        if split.len() > 1 {
            let res = split.iter().map(|s| s.parse::<i64>()
                .error_info("parsing")).collect::<RgResult<Vec<i64>>>();
            if let Ok(r) = res {
                servers = servers.iter().filter(|s| r.contains(&s.index)).cloned().collect_vec();
            }
        }
    }
    servers = servers.iter().filter(|s| s.network_environment.to_lowercase() == net.to_std_string() ||
        s.network_environment.to_lowercase() == "all"
    ).cloned().collect_vec();

    let mut peer_id_index: HashMap<i64, String> = HashMap::default();

    let mut pid_tx: HashMap<String, Transaction> = HashMap::default();
    let nc = Box::new(node_config.clone());

    for (ii, ss) in servers.iter().enumerate() {
        if let Some(i) = deploy.exclude_server_index {
            if ii == i as usize {
                continue;
            }
        }
        let dpl_tuple = deployment.as_ref()
            .and_then(|d| d.by_index(ii as i64));

        let opt_peer_id: Option<String> = peer_id_index.get(&ss.peer_id_index).cloned();
        let (words, peer_id_hex) = derive_mnemonic_and_peer_id(
            node_config,
            m.clone(),
            ss.peer_id_index as usize,
            deploy.cold,
            passphrase.clone(),
            opt_peer_id,
            ss.index,
            servers.clone(),
            vec![],
            &mut pid_tx,
            &net,
            ss.reward_address.clone()
        ).await?;

        let mut peer_tx_opt: Option<structs::Transaction> = None;
        let mut words_opt = if deploy.words || deploy.words_and_id {
            Some(words.clone())
        } else {
            None
        };
        let mut peer_id_hex_opt = if deploy.peer_id  || deploy.words_and_id {
            peer_tx_opt = pid_tx.get(&peer_id_hex).clone().cloned();
            Some(peer_id_hex.clone())
        } else {
            None
        };
        let _pid_tx_ser = if deploy.peer_id  || deploy.words_and_id {
            Some(pid_tx.clone())
        } else {
            None
        };
        peer_id_index.insert(ss.peer_id_index, peer_id_hex.clone());
        let hm = hm.clone();
        // println!("Setting up server: {}", ss.host.clone());

        if let Some(o) = &deploy.server_offline_info {
            let p = PathBuf::from(o);
            let pi = p.join(format!("{}", ss.index));
            let o = pi.join("peer_tx");
            let peer_ser = std::fs::read_to_string(o).expect("offline info");
            let peer_tx =  peer_ser.json_from::<Transaction>().expect("peer tx");
            peer_tx_opt = Some(peer_tx.clone());
            peer_id_hex_opt = Some(peer_tx.peer_data().expect("").peer_id.expect("").hex());
            let words_path = pi.join("mnemonic");
            let words_read = std::fs::read_to_string(words_path).expect("offline info");
            words_opt = Some(words_read);
        }

        restore_multiparty_share(node_config.clone(), ss.clone()).await?;

        // let ssh = SSH::new_ssh(ss.host.clone(), None);
        let ssh = DeployMachine::new(ss, None, output_handler.clone());
        if !deploy.skip_redgold_process {

            let this_hm = hm.clone();

            if ss.host == "".to_string() {
                let ssh_local = LocalSSHLike::new(output_handler.clone());
                let ssh = DeployMachine {
                    server: ss.clone(),
                    ssh: ssh_local,
                };
                let _t = tokio::time::timeout(Duration::from_secs(600), deploy_redgold(
                    ssh, net, gen.clone(), Some(this_hm.clone()), purge,
                    words_opt.clone(),
                    peer_id_hex_opt.clone(),
                    !deploy.debug_skip_start,
                    ss.node_name.clone(),
                    peer_tx_opt.clone().map(|p| p.json_or()),
                    &output_handler,
                    true,
                    node_config.config_data.clone(),
                    dpl_tuple.clone(),
                    &nc
                )).await.error_info("Timeout")??;
            }
            let _t = tokio::time::timeout(Duration::from_secs(600), deploy_redgold(
                ssh, net, gen, Some(this_hm.clone()), purge,
                words_opt.clone(),
                peer_id_hex_opt.clone(),
                !deploy.debug_skip_start,
                ss.node_name.clone(),
                peer_tx_opt.clone().map(|p| p.json_or()),
                &output_handler,
                deploy.disable_apt_system_init,
                node_config.config_data.clone(),
                dpl_tuple,
                &nc
            )).await.error_info("Timeout")??;
        }
        gen = false;
        if deploy.ops {
            let node_exporter_template = if ss.index == 0 {
                Some(
                    servers_original.iter().filter(|s| s.index != 0)
                        .cloned().collect_vec()
                )
            }  else {
                None
            };
            let ssh = DeployMachine::new(ss, None, output_handler.clone());
            let grafana_password = env::var("GRAFANA_PASSWORD").ok();
            deploy_ops_services(
                ssh, None, None, grafana_password, deploy.purge_ops,
                &output_handler, deploy.debug_skip_start,
                node_exporter_template,
                deploy.skip_logs,
                true,
                false
            ).await?
        }
    }
    Ok(())
}

//
// #[ignore]
// #[tokio::test]
// async fn test_setup_server() {
//     default_deploy().await;
// }
