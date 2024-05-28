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
pub struct NodeInstance {
    // Enable multiparty support, requires API keys and additional setup for oracle pricing info.
    pub name: String,
    pub index: i64,
    pub peer_id_index: i64,
    pub network_environment: NetworkEnvironment,
    pub external_host: Option<String>,
    pub host_port_offset: Option<i64>,
    pub vpn_config: Option<VPNConfig>
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ServerData {
    pub ssh_host: String,
    pub external_ipv4: Option<String>,
    pub instances: Vec<NodeInstance>
}

// Migrate node_config stuff here
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ServerConfigData {
    pub servers: Vec<ServerData>
}

impl Default for ServerConfigData {
    fn default() -> Self {
        Self {
            servers: vec![],
        }
    }
}
