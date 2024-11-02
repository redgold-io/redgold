use serde::{Deserialize, Serialize};
use crate::conf::local_stored_state::LocalStoredState;

#[derive(Clone, Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
#[serde(default)] // This allows fields to be omitted in TOML
pub struct DebugSettings {
    pub use_e2e_external_resource_mocks: bool,
    pub test: Option<String>
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
#[serde(default)] // This allows fields to be omitted in TOML
pub struct PartyConfigData {
    // Enable multiparty support, requires API keys and additional setup for oracle pricing info.
    pub enable_party_mode: bool,
    pub order_cutoff_delay_time: i64,
    pub poll_interval: i64,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
#[serde(default)]
pub struct PortfolioFulfillmentConfigData {
    pub stake_control_address: Option<String>
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
#[serde(default)]
pub struct ServiceIntervals {
    pub portfolio_fulfillment_agent_seconds: Option<u64>
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
#[serde(default)]
pub struct NodeData {
    pub words: Option<String>,
    pub peer_id: Option<String>,
    pub network: Option<String>,
    pub disable_control_api: Option<bool>,
    pub nat_traversal_required: Option<bool>,
    pub udp_keepalive_seconds: Option<u64>,
    pub service_intervals: Option<ServiceIntervals>,
    pub server_index: Option<i64>,
    pub port_offset: Option<i64>,
    pub passive: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
#[serde(default)]
pub struct SecureData {
    pub salt: Option<String>,
    pub session_salt: Option<String>,
    pub session_hashed_password: Option<String>,
    pub config: Option<String>,
    pub path: Option<String>,
    pub usb_paths: Option<Vec<String>>,
    pub capture_device_name: Option<String>
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
#[serde(default)] // This allows fields to be omitted in TOML
pub struct ExternalResources {
    pub s3_backup_bucket: Option<String>
}


#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
#[serde(default)] // This allows fields to be omitted in TOML
pub struct Keys {
    words: Option<String>,
    aws_access: Option<String>,
    aws_secret: Option<String>
}


// Migrate node_config stuff here
#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(default)] // This allows fields to be omitted in TOML
pub struct ConfigData {
    pub network: Option<String>,
    pub home: Option<String>,
    pub config: Option<String>,
    pub data: Option<String>,
    pub bulk: Option<String>,
    pub node: Option<NodeData>,
    pub party: Option<PartyConfigData>,
    pub debug: Option<DebugSettings>,
    pub local: Option<LocalStoredState>,
    pub portfolio: Option<PortfolioFulfillmentConfigData>,
    pub secure: Option<SecureData>,
    pub offline: Option<bool>,
    pub external: Option<ExternalResources>,
    pub keys: Option<Keys>
}

use std::env;
use crate::structs::NetworkEnvironment;

fn get_home_dir() -> Option<String> {
    env::var("HOME").or_else(|_| env::var("USERPROFILE")).ok()
}

impl Default for ConfigData {
    fn default() -> Self {
        Self {
            network: Some(NetworkEnvironment::Main.to_std_string()),
            home: get_home_dir(),
            config: None,
            data: None,
            bulk: None,
            node: None,
            party: Some(PartyConfigData {
                enable_party_mode: false,
                order_cutoff_delay_time: 300_000,
                poll_interval: 300_000,
            }),
            debug: Some(DebugSettings {
                use_e2e_external_resource_mocks: false,
                test: None,
            }),
            local: Default::default(),
            portfolio: Default::default(),
            secure: Default::default(),
            offline: None,
            external: None,
            keys: None,
        }
    }
}
