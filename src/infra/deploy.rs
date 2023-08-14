 use std::collections::HashMap;
 use std::env::VarError;
 use std::fs::File;
use redgold_schema::structs::{ErrorInfo, NetworkEnvironment, PeerId};
use std::io::{Write, Read, Seek, SeekFrom};
use std::thread::sleep;
use std::time::Duration;
use crate::infra::SSH;
use crate::resources::Resources;
// use filepath::FilePath;
use itertools::Itertools;
 use redgold_schema::RgResult;
 use redgold_keys::util::mnemonic_support::WordsPass;
 use crate::hardware::trezor;
 use crate::node_config::NodeConfig;
 use crate::util::cli::arg_parse_config::ArgTranslate;
 use crate::util::cli::args::{Deploy, RgTopLevelSubcommand};
 use crate::util::cli::commands::get_input;
 use crate::util::cli::data_folder::DataFolder;

 /**
Updates to this cannot be explicitly watched through docker watchtower for automatic updates
They must be manually deployed.

 This whole thing should really have a streaming output for the lines and stuff.
 */
pub async fn setup_server_redgold(
     mut ssh: SSH,
     network: NetworkEnvironment,
     is_genesis: bool,
     additional_env: Option<HashMap<String, String>>,
     purge_data: bool,
     words: Option<String>,
     peer_id_hex: Option<String>,
 ) -> Result<(), ErrorInfo> {

    ssh.verify()?;


     let host = ssh.host.clone();
     let p= &Box::new(move |s: String| {
         println!("{} output: {}", host.clone(), s);
         Ok::<(), ErrorInfo>(())
     });

    ssh.exes("docker system prune -a -f", p).await?;
    ssh.exes("apt install -y ufw", p).await?;
    ssh.exes("sudo ufw allow ssh", p).await?;
    ssh.exes("sudo ufw allow in on tailscale0", p).await?;
    ssh.exes("echo 'y' | sudo ufw enable", p).await?;

    let compose = ssh.exec("docker-compose", true);
    if !(compose.stderr.contains("applications")) {
        ssh.exes("curl -fsSL https://get.docker.com -o get-docker.sh; sh ./get-docker.sh", p).await?;
        ssh.exes("sudo apt install -y docker-compose", p).await?;
    }
    let r = Resources::default();

    let path = format!("/root/.rg/{}", network.to_std_string());
    let all_path = format!("/root/.rg/{}", NetworkEnvironment::All.to_std_string());

     // Copy mnemonic / peer_id
     if let Some(words) = words {
         let remote = format!("{}/mnemonic", all_path);
         ssh.copy_p(words, remote, p).await?;
     }
     if let Some(peer_id_hex) = peer_id_hex {
         let remote = format!("{}/peer_id", path);
         ssh.copy_p(peer_id_hex, remote, p).await?;
     }


    // TODO: Investigate issue with tmpfile, not working
    // // let mut tmpfile: File = tempfile::tempfile().unwrap();
    // // write!(tmpfile, "{}", r.redgold_docker_compose).unwrap();
    // TODO: Also wget from github directly depending on security concerns -- not verified from checksum hash
    // Only should be done to override if the given exe is outdated.
    ssh.exes(format!("mkdir -p {}", path), p).await?;
    ssh.copy_p(r.redgold_docker_compose, format!("{}/redgold-only.yml", path), p).await?;

    let port = network.default_port_offset();
    let mut env = additional_env.unwrap_or(Default::default());
    env.insert("REDGOLD_NETWORK".to_string(), network.to_std_string());
    env.insert("REDGOLD_GENESIS".to_string(), is_genesis.to_string());
    env.insert("REDGOLD_METRICS_PORT".to_string(), format!("{}", port - 1));
    env.insert("REDGOLD_P2P_PORT".to_string(), format!("{}", port));
    env.insert("REDGOLD_PUBLIC_PORT".to_string(), format!("{}", port + 1));
    env.insert("REDGOLD_CONTROL_PORT".to_string(), format!("{}", port + 2));

    let copy_env = vec!["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"];
    for e in copy_env {
        for i in std::env::var(e).ok() {
            env.insert(e.to_string(), i);
        }
    }

     // TODO: Lol not this
     let port_range: Vec<i64> = vec![-1, 0, 1, 4, 5, 6];
     for port_i in port_range {
         let port_o = (port as i64) + port_i;
         ssh.exes(format!("sudo ufw allow proto tcp from any to any port {}", port_o), p).await?;
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

    ssh.exes(format!("cd {}; docker-compose -f redgold-only.yml pull", path), p).await?;
    ssh.exes(format!("cd {}; docker-compose -f redgold-only.yml up -d", path), p).await?;
    ssh.exes("sudo ufw reload", p).await?;

    Ok(())
}

pub async fn setup_ops_services(
    mut ssh: SSH,
    _additional_env: Option<HashMap<String, String>>,
    remote_path_prefix: Option<String>,
    grafana_pass: Option<String>,
    purge_data: bool,
) -> Result<(), ErrorInfo> {
    let remote_path = remote_path_prefix.unwrap_or("/root/.rg/all".to_string());
    ssh.verify()?;

    let p = &Box::new(|s: String| {
        println!("Partial output: {}", s);
        Ok(())
    });

    ssh.execs("docker ps", false, p).await?;
    ssh.copy(
        include_str!("../resources/infra/ops_services/services-all.yml"),
        format!("{}/services-all.yml", remote_path)
    );
    ssh.copy(
        include_str!("../resources/infra/ops_services/filebeat.docker.yml"),
        format!("{}/filebeat.docker.yml", remote_path)
    );
    let prometheus_yml = include_str!("../resources/infra/ops_services/prometheus.yml").to_string();
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
    ssh.copy(
        prometheus_yml,
        format!("{}/prometheus.yml", remote_path)
    );
    ssh.copy(
        include_str!("../resources/infra/ops_services/prometheus-datasource.yaml"),
        format!("{}/prometheus-datasource.yaml", remote_path)
    );

    ssh.copy(
        grafana_pass.unwrap_or("debug".to_string()),
        format!("{}/grafana_password", remote_path)
    );

    ssh.execs(format!("rm -r {}/dashboards", remote_path), false, p).await?;
    ssh.execs(format!("mkdir {}/dashboards", remote_path), false, p).await?;

    let x = include_str!("../resources/infra/ops_services/dashboards/node-exporter-full_rev31.json");
    ssh.copy(
        x,
        format!("{}/dashboards/node-exporter.json", remote_path)
    );

    let x = include_str!("../resources/infra/ops_services/dashboards/redgold_rev0.json");
    ssh.copy(
        x,
        format!("{}/dashboards/redgold.json", remote_path)
    );

    // println!("Copying node exporter dashboard: {}", x);

    ssh.copy(
        include_str!("../resources/infra/ops_services/dashboards/dashboard_config.yaml"),
        format!("{}/dashboards/dashboard_config.yaml", remote_path)
    );

    ssh.copy(
        include_str!("../resources/infra/ops_services/grafana/grafana.ini"),
        format!("{}/grafana.ini", remote_path)
    );

    // Environment
    let mut env = _additional_env.unwrap_or(Default::default());
    env.insert("GF_SECURITY_ADMIN_PASSWORD__FILE".to_string(), "/etc/grafana/grafana_secret".to_string());
    let copy_env = vec!["SMTP_HOST", "SMTP_USER", "SMTP_PASSWORD", "SMTP_FROM_ADDRESS", "SMTP_FROM_NAME"];
    for e in copy_env {
        for i in std::env::var(e).ok() {
            env.insert(e.to_string(), i);
        }
    }
    let env_contents = env.iter().map(|(k, v)| {
        format!("{}={}", k, format!("{}", v))
    }).join("\n");
    ssh.copy(env_contents.clone(), format!("{}/ops_var.env", remote_path));

    ssh.execs(format!("cd {}; docker-compose -f services-all.yml down", remote_path), false, p).await?;

    for s in vec!["grafana", "prometheus", "esdata"] {
        if purge_data {
            ssh.execs(format!("rm -r {}/data/{}", remote_path, s), false, p).await?;
        }
        ssh.execs(format!("mkdir -p {}/data/{}", remote_path, s), false, p).await?;
    };

    ssh.exes(format!("chmod -R 777 {}/data/esdata", remote_path), p).await?;

    ssh.execs(format!("cd {}; docker-compose -f services-all.yml up -d", remote_path), false, p).await?;

    Ok(())
}


pub async fn derive_mnemonic_and_peer_id(
    mnemonic: String, peer_id_index: usize, cold: bool, passphrase: Option<String>,
    opt_peer_id: Option<String>,
    server_id_index: i64
)
    -> RgResult<(String, String)> {

    let w = WordsPass::new(mnemonic, passphrase);
    let new = w.hash_derive_words(server_id_index.to_string())?;
    let server_mnemonic = new.words;
    let account = (99 - peer_id_index) as u32;
    let mut pid_hex = "".to_string();
    if let Some(pid) = opt_peer_id {
        pid_hex = pid;
    } else {
        let pk = if cold {
            trezor::get_standard_public_key(
                account, None, 0, 0)?
        } else {
            w.public_at(format!("m/44'/0'/{}/0/0", account))?
        };
        pid_hex = pk.hex()?;
    }

    Ok((server_mnemonic, pid_hex))
}

pub async fn default_deploy(deploy: &mut Deploy, node_config: &NodeConfig) -> RgResult<()> {

    let primary_gen = std::env::var("REDGOLD_PRIMARY_GENESIS").is_ok();
    if primary_gen {
        // Also set environment here to dev if not main
        deploy.skip_ops = true;
    }
    let sd = ArgTranslate::secure_data_path_buf().expect("");
    let sd = sd.join(".rg");
    let df = DataFolder::from_path(sd);
    let buf = df.all().servers_path();
    let m = df.all().mnemonic().await.expect("");
    let passphrase = if deploy.ask_pass {
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
    };
    // Ok heres what to do, in here we need to potentially invoke the HW signer for peer id
    // if we don't have one generated FOR THE ENVIRONMENT of interest.
    // So check to see if the peer id exists, if not, generate it according to hardware signer
    // ONLY IF mainnet do we use hardware signer?
    //WordsPass::new(m)
    println!("Reading servers file: {:?}", buf);
    let s = ArgTranslate::read_servers_file(buf).expect("servers");
    println!("Setting up servers: {:?}", s);
    // let mut gen = true;
    let purge = deploy.purge;
    let mut gen = deploy.genesis;
    if primary_gen {
        gen = true;
    }
    let mut hm = HashMap::new();
    hm.insert("RUST_BACKTRACE".to_string(), "1".to_string());

    let mut net = node_config.network;
    if net == NetworkEnvironment::Local {
        net = NetworkEnvironment::Dev;
    } else {
        if node_config.opts.network.is_none() {
            if primary_gen {
                net = NetworkEnvironment::Dev;
            } else {
                // TODO Enable this when mainnet
                // net = NetworkEnvironment::Main;
            }
        }
        // Get node_config arg translate and set to dev if arg not supplied.
    }
    let mut servers = s.to_vec();
    if let Some(i) = deploy.server_index {
        let x = servers.get(i as usize).expect("").clone();
        servers = vec![x]
    }

    let mut peer_id_index: HashMap<i64, String> = HashMap::default();

    for (ii, ss) in servers.iter().enumerate() {
        if let Some(i) = deploy.exclude_server_index {
            if ii == i as usize {
                continue;
            }
        }

        let opt_peer_id: Option<String> = peer_id_index.get(&ss.peer_id_index).cloned();
        let (words, peer_id_hex) = derive_mnemonic_and_peer_id(
            m.clone(), ss.peer_id_index as usize, deploy.cold, passphrase.clone(), opt_peer_id,
            ss.index
        ).await?;
        let words_opt = if deploy.words || deploy.words_and_id {
            Some(words.clone())
        } else {
            None
        };
        let peer_id_hex_opt = if deploy.peer_id  || deploy.words_and_id {
            Some(peer_id_hex.clone())
        } else {
            None
        };
        peer_id_index.insert(ss.peer_id_index, peer_id_hex.clone());
        let hm = hm.clone();
        println!("Setting up server: {}", ss.host.clone());
        let ssh = SSH::new_ssh(ss.host.clone(), None);
        if !deploy.ops {
            setup_server_redgold(
                ssh, net, gen, Some(hm), purge,
                words_opt,
                peer_id_hex_opt,
            ).await.expect("worx");
        }
        gen = false;
        if !deploy.skip_ops {
            let ssh = SSH::new_ssh(ss.host.clone(), None);
            setup_ops_services(ssh, None, None, None, deploy.purge_ops).await.expect("")
        }
    }
    Ok(())
}


#[ignore]
#[tokio::test]
async fn test_setup_server() {
    // default_deploy().await;
}