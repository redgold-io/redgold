use serde::{Deserialize, Serialize};
use crate::local_stored_state::LocalStoredState;

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(default)] // This allows fields to be omitted in TOML
pub struct DebugSettings {
    pub use_e2e_external_resource_mocks: bool
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(default)] // This allows fields to be omitted in TOML
pub struct PartyConfigData {
    // Enable multiparty support, requires API keys and additional setup for oracle pricing info.
    pub enable_party_mode: bool,
    pub order_cutoff_delay_time: i64,
    pub poll_interval: i64,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(default)]
pub struct PortfolioFulfillmentConfigData {
    pub stake_control_address: Option<String>
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(default)]
pub struct ServiceIntervals {
    pub portfolio_fulfillment_agent_seconds: Option<u64>
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(default)]
pub struct NodeData {
    pub words: Option<String>,
    pub peer_id: Option<String>,
    pub network: Option<String>,
    pub disable_control_api: Option<bool>,
    pub nat_traversal_required: Option<bool>,
    pub udp_keepalive_seconds: Option<u64>,
    pub service_intervals: Option<ServiceIntervals>
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(default)]
pub struct SecureData {
    salt_mnemonic: Option<String>,
    session_salt: Option<String>,
    session_hashed_password: Option<String>,
}

// Migrate node_config stuff here
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default)] // This allows fields to be omitted in TOML
pub struct ConfigData {
    pub node_data: Option<NodeData>,
    pub party_config_data: Option<PartyConfigData>,
    pub debug_settings: Option<DebugSettings>,
    pub local_stored_state: Option<LocalStoredState>,
    pub portfolio_fulfillment_config_data: Option<PortfolioFulfillmentConfigData>,
    pub secure_data: Option<SecureData>,
}

impl Default for ConfigData {
    fn default() -> Self {
        Self {
            node_data: Some(NodeData {
                words: None,
                peer_id: None,
                network: None,
                disable_control_api: None,
                nat_traversal_required: None,
                udp_keepalive_seconds: None,
                service_intervals: Some(ServiceIntervals {
                    portfolio_fulfillment_agent_seconds: Some(3600*12),
                }),
            }),
            party_config_data: Some(PartyConfigData {
                enable_party_mode: false,
                order_cutoff_delay_time: 300_000,
                poll_interval: 300_000,
            }),
            debug_settings: Some(DebugSettings {
                use_e2e_external_resource_mocks: false,
            }),
            local_stored_state: Default::default(),
            portfolio_fulfillment_config_data: Default::default(),
            secure_data: Default::default(),
        }
    }
}
