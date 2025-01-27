use serde::{Deserialize, Serialize};
use crate::config_data::Keys;
use crate::servers::ServerOldFormat;
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
    proxy_external_ipv4: String,
    proxy_host: String,
}


#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct MultisigContract {
    network: String,
    address: String,
    currency: String
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct NodePartyConfig {
    contracts: Option<Vec<MultisigContract>>
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct NodeInstance {
    // Enable multiparty support, requires API keys and additional setup for oracle pricing info.
    pub name: Option<String>,
    pub index: Option<i64>,
    pub peer_id_index: Option<i64>,
    pub network_environment: Option<String>,
    pub host_port_offset: Option<i64>,
    pub docker_swarm_proxy: Option<String>,
    pub keys: Option<Keys>,
    pub reward_address: Option<String>,
    pub use_id_ds_prefix: Option<bool>,
    pub party_config: Option<NodePartyConfig>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
pub struct ServerData {
    pub ssh_host: Option<String>,
    pub ssh_user: Option<String>,
    pub is_localhost: Option<bool>,
    pub external_ipv4: Option<String>,
    pub external_hostname: Option<String>,
    pub instances: Option<Vec<NodeInstance>>,
    pub deploy_metrics_instance: Option<bool>,
    pub ssh_jump_host: Option<String>
}
#[derive(Clone, Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
pub struct DeploymentDefaultParams {
    pub reward_address: Option<String>,
    pub keys: Option<Keys>,
    pub network_environment: Option<String>
    // pub deploy_metrics_instance: Option<bool>
}
// Migrate node_config stuff here
#[derive(Clone, Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
pub struct Deployment {
    pub servers: Option<Vec<ServerData>>,
    pub docker_swarm_proxies: Option<Vec<DockerSwarmProxy>>,
    pub default_params: Option<DeploymentDefaultParams>,
}

impl Deployment {
    pub fn as_old_servers(&self) -> Vec<ServerOldFormat> {
        let mut servers = vec![];
        let mut id = 0;
        let reward = self.default_params.as_ref().and_then(|d| d.reward_address.clone());
        if let Some(s) = &self.servers {
            for server in s.iter() {
                if let Some(instances) = &server.instances {
                    for instance in instances {
                        let mut server_old = ServerOldFormat::default();
                        server_old.host = server.ssh_host.clone().unwrap_or_default();
                        server_old.name = instance.name.clone().unwrap_or("".to_string());
                        server_old.index = instance.index.unwrap_or(id);
                        server_old.peer_id_index = instance.peer_id_index.unwrap_or(id);
                        server_old.network_environment = instance.network_environment.clone().unwrap_or("all".to_string());
                        server_old.node_name = instance.name.clone();
                        server_old.external_host = server.external_hostname.clone();
                        server_old.ipv4 = server.external_ipv4.clone();
                        server_old.username = server.ssh_user.clone();
                        server_old.reward_address = instance.reward_address.clone().or(reward.clone());
                        server_old.jump_host = server.ssh_jump_host.clone();
                        servers.push(server_old);
                        id += 1;
                    }
                }
            }
        }
        servers
    }
    pub fn fill_params(mut self) -> Self {
        let mut id = 0;
        let reward = self.default_params.as_ref().and_then(|d| d.reward_address.clone());
        if let Some(s) = self.servers.as_mut() {
            for server in s.iter_mut() {
                if let Some(instances) = server.instances.as_mut() {
                    for instance in instances {
                        if instance.index.is_none() {
                            instance.index = Some(id);
                        }
                        if instance.peer_id_index.is_none() {
                            instance.peer_id_index = Some(id);
                        }
                        instance.reward_address = instance.reward_address.clone().or(reward.clone());
                        id += 1;
                    }
                }
            }
        }
        self
    }

    pub fn by_index(&self, index: i64) -> Option<(Self, ServerData, NodeInstance)> {
        if let Some(s) = self.servers.as_ref() {
            for server in s.iter() {
                if let Some(instances) = server.instances.as_ref() {
                    for instance in instances {
                        if let Some(iidx) = instance.index {
                            if iidx == index {
                                return Some((self.clone(), server.clone(), instance.clone()));
                            }
                        }
                    }
                }
            }
        }
        None
    }
}