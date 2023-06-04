 use std::collections::HashMap;
 use std::env::VarError;
 use std::fs::File;
use redgold_schema::structs::{ErrorInfo, NetworkEnvironment};
use std::io::{Write, Read, Seek, SeekFrom};
use std::thread::sleep;
use std::time::Duration;
use crate::infra::SSH;
use crate::resources::Resources;
use filepath::FilePath;
use itertools::Itertools;
 use crate::node_config::NodeConfig;
 use crate::util::cli::arg_parse_config::ArgTranslate;
 use crate::util::cli::data_folder::DataFolder;

 /**
Updates to this cannot be explicitly watched through docker watchtower for automatic updates
They must be manually deployed.

 This whole thing should really have a streaming output for the lines and stuff.
 */
pub fn setup_server_redgold(
    mut ssh: SSH,
    network: NetworkEnvironment,
    is_genesis: bool,
    additional_env: Option<HashMap<String, String>>,
    purge_data: bool,
) -> Result<(), ErrorInfo> {

    ssh.verify()?;

    let compose = ssh.exec("docker-compose", true);
    if !(compose.stderr.contains("applications")) {
        ssh.run("curl -fsSL https://get.docker.com -o get-docker.sh; sh ./get-docker.sh");
        ssh.run("sudo apt install -y docker-compose");
    }
    let r = Resources::default();

    let path = format!("/root/.rg/{}", network.to_std_string());

    // TODO: Investigate issue with tmpfile, not working
    // // let mut tmpfile: File = tempfile::tempfile().unwrap();
    // // write!(tmpfile, "{}", r.redgold_docker_compose).unwrap();
    // TODO: Also wget from github directly depending on security concerns -- not verified from checksum hash
    // Only should be done to override if the given exe is outdated.
    ssh.exec(format!("mkdir -p {}", path), true);

    ssh.copy(r.redgold_docker_compose, format!("{}/redgold-only.yml", path));

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
    ssh.exec(format!("sudo ufw allow proto tcp from any to any port {}", port), true);
    ssh.exec(format!("sudo ufw allow proto tcp from any to any port {}", port - 1), true);
    ssh.exec(format!("sudo ufw allow proto tcp from any to any port {}", port + 1), true);
    ssh.exec(format!("sudo ufw allow proto tcp from any to any port {}", port + 4), true);
    ssh.exec(format!("sudo ufw allow proto udp from any to any port {}", port + 5), true);

    let env_contents = env.iter().map(|(k, v)| {
        format!("{}={}", k, format!("{}", v))
    }).join("\n");
    ssh.copy(env_contents.clone(), format!("{}/var.env", path));
    ssh.copy(env_contents, format!("{}/.env", path));

    sleep(Duration::from_secs(4));

    ssh.exec(format!("cd {}; docker-compose -f redgold-only.yml down", path), true);

    if purge_data {
        println!("Purging data");
        ssh.exec(format!("rm -rf {}/{}", path, "data_store.sqlite"), true);
    }

    ssh.exec(format!("cd {}; docker-compose -f redgold-only.yml pull", path), true);
    ssh.exec(format!("cd {}; docker-compose -f redgold-only.yml up -d", path), true);
    Ok(())
}

pub async fn setup_ops_services(
    mut ssh: SSH,
    _additional_env: Option<HashMap<String, String>>,
    remote_path_prefix: Option<String>,
    grafana_pass: Option<String>,
) -> Result<(), ErrorInfo> {
    let remote_path = remote_path_prefix.unwrap_or("/root/.rg/all".to_string());
    ssh.verify()?;

    let p = |s: String| {
        println!("Partial output: {}", s);
        Ok(())
    };

    ssh.stream_partial("docker ps", false, p).await?;
    ssh.copy(
        include_str!("../resources/infra/ops_services/services-all.yml"),
        format!("{}/services-all.yml", remote_path)
    );
    ssh.copy(
        include_str!("../resources/infra/ops_services/filebeat.docker.yml"),
        format!("{}/filebeat.docker.yml", remote_path)
    );
    let promtheus_yml = include_str!("../resources/infra/ops_services/prometheus.yml").to_string();
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
        promtheus_yml,
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

    ssh.stream_partial(format!("rm -r {}/dashboards", remote_path), false, p).await?;
    ssh.stream_partial(format!("mkdir {}/dashboards", remote_path), false, p).await?;

    let x = include_str!("../resources/infra/ops_services/dashboards/node-exporter-full_rev31.json");
    ssh.copy(
        x.clone(),
        format!("{}/dashboards/node-exporter.json", remote_path)
    );

    // println!("Copying node exporter dashboard: {}", x);

    ssh.copy(
        include_str!("../resources/infra/ops_services/dashboards/dashboard_config.yaml"),
        format!("{}/dashboards/dashboard_config.yaml", remote_path)
    );

    ssh.stream_partial(format!("cd {}; docker-compose -f services-all.yml down", remote_path), false, p).await?;

    for s in vec!["grafana", "prometheus", "esdata"] {
        ssh.stream_partial(format!("rm -r {}/data/{}", remote_path, s), false, p).await?;
        ssh.stream_partial(format!("mkdir {}/data/{}", remote_path, s), false, p).await?;
    };

    ssh.stream_partial(format!("cd {}; docker-compose -f services-all.yml up -d", remote_path), false, p).await?;

    Ok(())
}


#[ignore]
#[tokio::test]
async fn test_setup_server() {
    // sudo ufw allow proto tcp from any to any port 16181
        // sudo ufw allow proto tcp from any to any port 16180

    //
    //
    // let ssh = SSH::new_ssh("hostnoc.redgold.io", None);
    // setup_ops_services(ssh, None, None, None).await.expect("");
    // setup_server_redgold(ssh, NetworkEnvironment::Predev, true, None, true).expect("worx");
    //
    // let ssh = SSH::new_ssh("interserver.redgold.io", None);
    // setup_server_redgold(ssh, NetworkEnvironment::Predev, false, None, true).expect("worx");
    // let mut nc = NodeConfig::default();
    let sd = ArgTranslate::secure_data_path_buf().expect("");
    let sd = sd.join(".rg");
    let df = DataFolder::from_path(sd);
    let buf = df.all().servers_path();
    println!("Reading servers file: {:?}", buf);
    let s = ArgTranslate::read_servers_file(buf).expect("servers");
    println!("Setting up servers: {:?}", s);
    let mut gen = true;
    for ss in s {
        println!("Setting up server: {}", ss.host.clone());
        let ssh = SSH::new_ssh(ss.host, None);
        setup_server_redgold(ssh, NetworkEnvironment::Dev, gen, None, true).expect("worx");
        gen = false
    }
    // df.all().data_store().config_store
}