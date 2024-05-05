use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PartyConfigData {
    // Enable multiparty support, requires API keys and additional setup for oracle pricing info.
    pub enable_party_mode: bool,
    pub order_cutoff_delay_time: i64,
    pub poll_interval: i64,
}

// Migrate node_config stuff here
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ConfigData {
    pub party_config_data: PartyConfigData
}

impl Default for ConfigData {
    fn default() -> Self {
        Self {
            party_config_data: PartyConfigData {
                enable_party_mode: false,
                order_cutoff_delay_time: 300_000,
                poll_interval: 300_000,
            },
        }
    }
}
