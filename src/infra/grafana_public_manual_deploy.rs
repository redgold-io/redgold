use redgold_schema::servers::Server;
use crate::infra::deploy::{deploy_ops_services, DeployMachine, SSHProcessInvoke};
use crate::node_config::NodeConfig;
use crate::util::cli::commands::log_handler;


#[ignore]
#[tokio::test]
async fn manual() {
    let n = NodeConfig::dev_default().await;
    let servers = n.secure_data_folder.expect("works").all().servers().expect("works");
    let s = Server::new("grafana".to_string());

    let (default_fun, output_handler) = log_handler();
    let mut ssh = DeployMachine::new(&s, None, output_handler.clone());

    let p = &output_handler.clone();
    // ssh.exes("apt install -y ufw", p).await.expect("");
    // ssh.exes("sudo ufw allow ssh", p).await.expect("");
    // ssh.exes("sudo ufw allow in on tailscale0", p).await.expect("");
    // ssh.exes("echo 'y' | sudo ufw enable", p).await.expect("");
    // let port_o = 3000;
    // ssh.exes(format!("sudo ufw allow proto tcp from any to any port {}", port_o), p).await.expect("");
    // ssh.exes("sudo ufw reload", p).await.expect("");
    let mut d = DeployMachine::new(&s, None, output_handler.clone());
    d.install_docker(&output_handler).await.expect("works");

    deploy_ops_services(
        d,
        None,
        None,
        Some(std::env::var("GRAFANA_PASSWORD").expect("works")),
        false,
        &output_handler,
        false,
        Some(servers),
        true,
        false,
        true
    ).await.expect("works");

}