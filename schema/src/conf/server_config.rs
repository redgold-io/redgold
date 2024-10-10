use serde::{Deserialize, Serialize};
use crate::structs::NetworkEnvironment;


#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VPNConfig {
    user: String,
    password: String,
    provider: String,
    server: String
}


#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct HAProxyConfig {
    proxy_external_ip: String,
    proxy_host: String,
}


#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct DockerSwarmProxy {
    name: String,
    proxy_external_ip: String,
    proxy_host: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct NodeInstance {
    // Enable multiparty support, requires API keys and additional setup for oracle pricing info.
    pub name: String,
    pub index: i64,
    pub peer_id_index: i64,
    pub network_environment: NetworkEnvironment,
    pub host_port_offset: Option<i64>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
pub struct ServerData {
    pub ssh_host: Option<String>,
    pub is_localhost: Option<bool>,
    pub external_ipv4: Option<String>,
    pub external_hostname: Option<String>,
    pub instances: Option<Vec<NodeInstance>>,
    pub deploy_metrics_instance: Option<bool>
}

// Migrate node_config stuff here
#[derive(Clone, Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
pub struct Deployment {
    pub servers: Option<Vec<ServerData>>,
    pub docker_swarm_proxies: Option<Vec<DockerSwarmProxy>>
}

