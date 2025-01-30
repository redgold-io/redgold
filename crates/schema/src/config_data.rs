use crate::conf::local_stored_state::{AccountKeySource, LocalStoredState};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
#[serde(default)] // This allows fields to be omitted in TOML
pub struct DebugSettings {
    pub use_e2e_external_resource_mocks: Option<bool>,
    pub test: Option<String>,
    // dev mode
    pub develop: Option<bool>,
    // main developer
    pub developer: Option<bool>,
    pub id: Option<i32>,
    pub genesis: Option<bool>,
    pub enable_live_e2e: Option<bool>,
    pub grafana_writer: Option<bool>,
    pub live_e2e_interval_seconds: Option<i64>,
    pub bypass_seed_enrichment: Option<bool>,
    pub bypass_download: Option<bool>,
    // CI / Debug words
    pub words: Option<String>
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, Eq, PartialEq)]
#[serde(default)] // This allows fields to be omitted in TOML
pub struct PartyConfigData {
    // Enable multiparty support, requires API keys and additional setup for oracle pricing info.
    pub enable: Option<bool>,
    pub order_cutoff_delay_time: Option<i64>,
    pub poll_interval: Option<i64>,
    pub peer_timeout_seconds: Option<i64>,
    pub gg20_peer_timeout_seconds: Option<i64>,
    pub party_config: Option<NodePartyConfig>,
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
pub struct DaqConfig {
    pub poll_duration_seconds: Option<i64>
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
    pub peer_id_index: Option<i64>,
    pub port_offset: Option<i64>,
    // Doesn't participate in any consensus operations, only watches the network for live data
    pub watch_only_node: Option<bool>,
    // Warning, you must have a LOT of disk space available / S3 for this to work
    pub archival_mode: Option<bool>,
    pub name: Option<String>,
    pub ip: Option<String>,
    pub http_client_proxy: Option<String>,
    pub udp_serve_disabled: Option<bool>,
    pub allowed_http_proxy_origins: Option<Vec<String>>,
    // pub daq: Option<DaqConfig>
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
pub struct RpcUrl {
    pub currency: SupportedCurrency,
    pub url: String,
    pub network: String,
    pub wallet_only: Option<bool>,
    pub authentication: Option<String>,
    pub file_path: Option<String>,
    pub ws_only: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
#[serde(default)] // This allows fields to be omitted in TOML
pub struct ExternalResources {
    pub s3_backup_bucket: Option<String>,
    pub rpcs: Option<Vec<RpcUrl>>,
}


#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
#[serde(default)] // This allows fields to be omitted in TOML
pub struct Keys {
    pub words: Option<String>,
    pub aws_access: Option<String>,
    pub aws_secret: Option<String>,
    pub etherscan: Option<String>,
    pub recaptcha: Option<String>,
}


#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
#[serde(default)] // This allows fields to be omitted in TOML
pub struct EmailSettings {
    pub from: Option<String>,
    pub to: Option<String>,
}


// TODO: Consider should this be used as a global arg?
#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
#[serde(default)] // This allows fields to be omitted in TOML
pub struct CliSettings {
    pub cold: Option<bool>,
    pub airgap: Option<bool>,
    pub account: Option<String>,
    pub currency: Option<String>,
    pub path: Option<String>,
    pub verbose: Option<bool>,
    pub quiet: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
#[serde(default)] // This allows fields to be omitted in TOML
pub struct TrustRatingLabelUserInput {
    pub peer_id: String,
    pub trust_data: Vec<TrustData>
}

impl TrustRatingLabelUserInput {
    pub fn to_trust_rating_label(&self) -> RgResult<TrustRatingLabel> {
        Ok(TrustRatingLabel {
            peer_id: Some(PeerId::from_hex(&self.peer_id)?),
            trust_data: self.trust_data.clone(),
        })
    }
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
    pub keys: Option<Keys>,
    pub email: Option<EmailSettings>,
    pub cli: Option<CliSettings>
}

impl ConfigData {

    pub fn development_mode(&self) -> bool {
        self.debug.as_ref().and_then(|x| x.develop).unwrap_or(false)
    }

    #[allow(deprecated)]
    pub fn servers_old(&self) -> Vec<ServerOldFormat> {
        self.local.as_ref()
            .and_then(|x|
                          x.deploy.as_ref()
                              .map(|x| x.as_old_servers())
                              .or(x.servers.clone())
            ).unwrap_or_default()
    }

    #[allow(deprecated)]
    pub fn generate_user_sample_config() -> Self {
        Self {
            network: Some("main".to_string()),
            home: Some("/home/user".to_string()),
            config: Some("/home/user/.rg/config.toml".to_string()),
            data: Some("/home/user/.rg".to_string()),
            bulk: Some("/home/user/mnt/.rg/".to_string()),
            node: Some(NodeData {
                words: Some("abuse lock pledge crowd pair become ridge alone target viable black plate ripple sad tape victory blood river gloom air crash invite volcano release".to_string()),
                peer_id: Some("enter_your_peer_id_here_or_blank_to_generate_or_use_the_deploy_script".to_string()),
                network: Some("main".to_string()),
                disable_control_api: Some(false),
                nat_traversal_required: Some(false),
                udp_keepalive_seconds: None,
                service_intervals: None,
                server_index: Some(0),
                peer_id_index: Some(0),
                port_offset: Some(16180),
                watch_only_node: Some(false),
                archival_mode: Some(false),
                name: Some("your_node_name_if_set_manually_instead_of_through_deployment".to_string()),
                ip: Some("your_external_ip_goes_here".to_string()),
                http_client_proxy: None,
                udp_serve_disabled: Some(false),
                allowed_http_proxy_origins: None,
                // daq: None,
            }),
            party: Some(PartyConfigData {
                enable: Some(false),
                order_cutoff_delay_time: None,
                poll_interval: None,
                peer_timeout_seconds: None,
                gg20_peer_timeout_seconds: None,
                party_config: None,
            }),
            debug: None,
            local: Some(LocalStoredState {
                deploy: Some(Deployment{
                    servers: Some(vec![ServerData{
                        ssh_host: Some("your_ssh_host".to_string()),
                        ssh_user: Some("root".to_string()),
                        is_localhost: Some(false),
                        external_ipv4: Some("your_ssh_machines_external_ip_here".to_string()),
                        external_hostname: Some("only_necessary_if_using_dns".to_string()),
                        instances: Some(vec![
                            NodeInstance {
                                name: Some("your_node_name".to_string()),
                                index: Some(0),
                                peer_id_index: Some(0),
                                network_environment: Some("main".to_string()),
                                host_port_offset: None,
                                docker_swarm_proxy: None,
                                keys: None,
                                reward_address: None,
                                use_id_ds_prefix: None,
                                party_config: None,
                                rpc_overrides: None,
                            }
                        ]
                        ),
                        deploy_metrics_instance: Some(true),
                        ssh_jump_host: None,
                    }]),
                    docker_swarm_proxies: None,
                    default_params: Some(DeploymentDefaultParams{
                        reward_address: Some("your_cold_reward_address_here".to_string()),
                        keys: None,
                        network_environment: None,
                    }),
                }),
                servers: None,
                keys: Some(
                    vec![
                        AccountKeySource {
                            name: "your_xpub_name".to_string(),
                            derivation_path: "some_derivation_path".to_string(),
                            xpub: "your_xpub_here".to_string(),
                            hot_offset: None,
                            key_name_source: None,
                            device_id: None,
                            key_reference_source: None,
                            key_nickname_source: None,
                            request_type: None,
                            skip_persist: None,
                            preferred_address: None,
                            all_address: None,
                            public_key: None,
                        }
                    ]
                ),
                // TODO: Add a user input type for trust data to avoid direct conversions
                trust: None,
                // trust: Some(
                //     vec![
                //         ServerTrustRatingLabels {
                //             peer_id_index: 0,
                //             labels: vec![
                //                 TrustRatingLabelUserInput {
                //                     peer_id: "other_peer_id".to_string(),
                //                     trust_data: vec![
                //                         TrustData{
                //                             label_rating: Some(600),
                //                             confidence: Some(800),
                //                             hardness: Some(50),
                //                             data: None,
                //                             allow_model_override: true,
                //                             rating_type: Some(RatingType::SecurityRating as i32),
                //                         }
                //                     ],
                //                 }
                //             ],
                //             environment: Some("main".to_string()),
                //         }
                //     ]
                // ),
                saved_addresses: None,
                contacts: None,
                watched_address: None,
                email_alert_config: None,
                identities: None,
                mnemonics: None,
                private_keys: None,
                internal_stored_data: None,
            }),
            portfolio: None,
            secure: None,
            offline: None,
            external: None,
            keys: None,
            email: None,
            cli: None,
        }
    }
}

use crate::conf::server_config::{Deployment, DeploymentDefaultParams, NodeInstance, NodePartyConfig, ServerData};
use crate::proto_serde::ProtoSerde;
use crate::servers::ServerOldFormat;
use crate::structs::{PeerId, SupportedCurrency, TrustData, TrustRatingLabel};
use crate::RgResult;
use std::env;

fn get_home_dir() -> Option<String> {
    env::var("HOME").or_else(|_| env::var("USERPROFILE")).ok()
}

impl Default for ConfigData {
    fn default() -> Self {
        Self {
            network: None,
            home: get_home_dir(),
            config: None,
            data: None,
            bulk: None,
            node: None,
            party: None,
            debug: None,
            local: Default::default(),
            portfolio: Default::default(),
            secure: Default::default(),
            offline: None,
            external: None,
            keys: None,
            email: None,
            cli: None
        }
    }
}
