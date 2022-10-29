 use std::collections::HashMap;
use std::fs::File;
use redgold_schema::structs::{ErrorInfo, NetworkEnvironment};
use std::io::{Write, Read, Seek, SeekFrom};
use std::thread::sleep;
use std::time::Duration;
use crate::infra::SSH;
use crate::resources::Resources;
use filepath::FilePath;
use itertools::Itertools;


pub fn setup_services(
    mut ssh: SSH
) -> Result<(), ErrorInfo> {
    ssh.verify()?;
    Ok(())
}

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

    let mut env = additional_env.unwrap_or(Default::default());
    env.insert("REDGOLD_NETWORK".to_string(), network.to_std_string());
    env.insert("REDGOLD_GENESIS".to_string(), is_genesis.to_string());
    let env_contents = env.iter().map(|(k, v)| {
        format!("{}={}", k, format!("\"{}\"", v))
    }).join("\n");
    ssh.copy(env_contents, format!("{}/var.env", path));

    // sleep(Duration::from_secs(4));

    ssh.exec(format!("docker-compose -f {}/redgold-only.yml down", path), true);

    if purge_data {
        println!("Purging data");
        ssh.exec(format!("rm -rf {}/{}", path, "data_store.sqlite"), true);
    }

    // ssh.exec(format!("docker-compose -f {}/redgold-only.yml pull", path), true);
    // ssh.exec(format!("docker-compose -f {}/redgold-only.yml up -d", path), true);
    Ok(())
}


#[ignore]
#[test]
fn test_setup_server() {
    // sudo ufw allow proto tcp from any to any port 16181
        // sudo ufw allow proto tcp from any to any port 16180

    let ssh = SSH::new_ssh("hostnoc.redgold.io", None);
    setup_server_redgold(ssh, NetworkEnvironment::Dev, true, None, true).expect("worx");

    let ssh = SSH::new_ssh("interserver.redgold.io", None);
    setup_server_redgold(ssh, NetworkEnvironment::Dev, false, None, true).expect("worx");
}