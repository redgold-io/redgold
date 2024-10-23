
use std::time::Duration;
use tracing::info;
use std::path::PathBuf;
use std::fs;
use std::sync::Arc;
use itertools::Itertools;
use crate::config_data::ConfigData;
use crate::seeds::get_seeds_by_env_time;
use crate::servers::ServerOldFormat;
use crate::{structs, ErrorInfoContext, RgResult, ShortString};
use crate::conf::rg_args::{empty_args, RgArgs};
use crate::constants::{DEBUG_FINALIZATION_INTERVAL_MILLIS, OBSERVATION_FORMATION_TIME_MILLIS, REWARD_POLL_INTERVAL, STANDARD_FINALIZATION_INTERVAL_MILLIS};
use crate::data_folder::{DataFolder, EnvDataFolder};
use crate::observability::errors::Loggable;
use crate::proto_serde::ProtoSerde;
use crate::structs::{Address, DynamicNodeMetadata, ErrorInfo, NetworkEnvironment, NodeMetadata, NodeType, PeerId, PeerMetadata, PublicKey, Seed, Transaction, TransportInfo, TrustData, VersionInfo};
use crate::util::times::current_time_millis;

pub struct CanaryConfig {}

#[derive(Clone, Debug)]
pub struct GenesisConfig {
    // block: Block,
}

#[derive(Clone, Debug)]
pub struct MempoolConfig {
    pub channel_bound: usize,
    pub max_mempool_size: usize,
    pub max_mempool_age: Duration,
    pub allow_bypass: bool,
    pub interval: Duration
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            channel_bound: 1000,
            max_mempool_size: 100000,
            max_mempool_age: Duration::from_secs(3600),
            allow_bypass: true,
            interval: Duration::from_secs(1),
        }
    }
}

impl Default for TransactionProcessingConfig {
    fn default() -> Self {
        Self {
            channel_bound: 1000,
            concurrency: 100,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TransactionProcessingConfig {
    pub channel_bound: usize,
    pub concurrency: usize,
}

impl Default for ObservationConfig {
    fn default() -> Self {
        Self {
            channel_bound: 1000
        }
    }
}

#[derive(Clone, Debug)]
pub struct ObservationConfig {
    pub channel_bound: usize,
}

impl Default for ContractConfig {
    fn default() -> Self {
        Self {
            contract_state_channel_bound: 1000,
            bucket_parallelism: 10,
            interval: Duration::from_secs(1),
            ordering_delay: Duration::from_secs(1)
        }
    }
}

#[derive(Clone, Debug)]
pub struct ContractConfig {
    pub contract_state_channel_bound: usize,
    pub bucket_parallelism: usize,
    pub interval: Duration,
    pub ordering_delay: Duration,
}

impl Default for ContentionConfig {
    fn default() -> Self {
        Self {
            channel_bound: 1000,
            bucket_parallelism: 10,
            interval: Duration::from_secs(1),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ContentionConfig {
    pub channel_bound: usize,
    pub bucket_parallelism: usize,
    pub interval: Duration
}

#[derive(Clone, Debug)]
pub struct NodeInfoConfig {
    pub alias: Option<String>,
}

impl Default for NodeInfoConfig {
    fn default() -> Self {
        Self {
            alias: None,
        }
    }
}

// TODO: put the default node configs here
#[derive(Clone, Debug)]
pub struct NodeConfig {
    pub config_data: ConfigData,
    // User supplied params
    // TODO: Should this be a class Peer_ID with a multihash of the top level?
    // TODO: Review all schemas to see if we can switch to multiformats types.
    // pub self_peer_id: Vec<u8>,
    // Remove above and rename to peer_id -- this field is not in use yet.
    pub peer_id: PeerId,
    // This field is not used yet; it is a placeholder for future use.
    pub public_key: structs::PublicKey,
    // TODO: Change to Seed class? or maybe not leave it as it's own
    pub mnemonic_words: String,
    // Sometimes adjusted user params
    pub port_offset: u16,
    pub p2p_port: Option<u16>,
    pub control_port: Option<u16>,
    pub public_port: Option<u16>,
    pub rosetta_port: Option<u16>,
    pub disable_control_api: bool,
    pub disable_public_api: bool,
    // Rarely adjusted user suppliable params
    pub seed_hosts: Vec<String>,
    // Custom debug only network params
    pub observation_formation_millis: Duration,
    pub transaction_finalization_time: Duration,
    pub reward_poll_interval_secs: u64,
    pub network: NetworkEnvironment,
    pub check_observations_done_poll_interval: Duration,
    pub check_observations_done_poll_attempts: u64,
    pub seeds: Vec<Seed>,
    pub manually_added_seeds: Vec<Seed>,
    pub ignore_default_seeds: bool,
    pub executable_checksum: Option<String>,
    pub disable_auto_update: bool,
    pub auto_update_poll_interval: Duration,
    pub block_formation_interval: Duration,
    pub genesis_config: GenesisConfig,
    pub faucet_enabled: bool,
    pub e2e_enabled: bool,
    pub load_balancer_url: String,
    pub external_ip: String,
    pub external_host: String,
    pub servers: Vec<ServerOldFormat>,
    pub log_level: String,
    pub data_folder: DataFolder,
    pub secure_data_folder: Option<DataFolder>,
    pub enable_logging: bool,
    pub discovery_interval: Duration,
    pub shuffle_interval: Duration,
    pub live_e2e_interval: Duration,
    pub genesis: bool,
    pub opts: Arc<RgArgs>,
    pub mempool: MempoolConfig,
    pub tx_config: TransactionProcessingConfig,
    pub observation: ObservationConfig,
    pub contract: ContractConfig,
    pub contention: ContentionConfig,
    pub node_info: NodeInfoConfig,
    pub default_timeout: Duration,
    pub disable_metrics: bool,
    pub args: Arc<Vec<String>>,
    pub abort: bool,
    pub is_gui: bool
}

impl NodeConfig {

    pub fn portfolio_fulfillment_agent_duration(&self) -> Duration {
        let default = 3600 * 12;
        Duration::from_secs(
            self.config_data.node.as_ref()
                .and_then(|x| x.service_intervals.as_ref())
                .and_then(|x| x.portfolio_fulfillment_agent_seconds)
                .unwrap_or(default)
        )
    }

    pub fn s3_backup(&self) -> Option<&String> {
        self.config_data.external.as_ref().and_then(|e| e.s3_backup_bucket.as_ref())
    }

    pub fn server_index(&self) -> i64 {
        self.config_data.node.as_ref().and_then(|n| n.server_index).unwrap_or(0)
    }

    pub fn offline(&self) -> bool {
        self.config_data.offline.unwrap_or(false)
    }

    pub fn seed_peer_addresses(&self) -> Vec<Address> {
        self.seeds_now().iter()
            .flat_map(|s| s.peer_id.as_ref())
            .flat_map(|p| p.peer_id.as_ref())
            .flat_map(|p| p.address().ok())
            .collect_vec()
    }

    pub fn seed_node_addresses(&self) -> Vec<Address> {
        self.seeds_now().iter()
            .flat_map(|s| s.public_key.as_ref())
            .flat_map(|p| p.address().ok())
            .collect_vec()
    }

    pub fn seed_addresses_all(&self) -> Vec<Address> {
        self.seed_node_addresses().iter().chain(self.seed_peer_addresses().iter()).cloned().collect_vec()
    }

    pub fn seeds_at(&self, time: i64) -> Vec<Seed> {
        // TODO: Merge with CLI option seeds, allow disabling also
        let mut all_seeds = vec![];
        let hardcoded_seeds = get_seeds_by_env_time(&self.network, time);
        if !self.ignore_default_seeds {
            all_seeds.extend(hardcoded_seeds);
        }
        all_seeds.extend(self.seeds.clone());
        all_seeds.iter().unique().cloned().collect_vec()
    }

    pub fn seeds_at_pk(&self, time: i64) -> Vec<PublicKey> {
        self.seeds_at(time).iter().flat_map(|s| s.public_key.clone()).collect()
    }

    // This may cause a problem with manually configured seeds
    pub fn seeds_now(&self) -> Vec<Seed> {
        self.seeds_at(current_time_millis())
    }

    pub fn seeds_now_pk(&self) -> Vec<PublicKey> {
        self.seeds_now().iter().flat_map(|s| s.public_key.as_ref()).cloned().collect()
    }

    pub fn is_seed(&self, pk: &PublicKey) -> bool {
        self.seeds_now().iter().filter(|&s| s.public_key.as_ref() == Some(pk)).next().is_some()
    }

    pub fn seeds_pk(&self) -> Vec<structs::PublicKey> {
        self.seeds_now().iter().flat_map(|s| s.public_key.clone()).collect()
    }

    pub fn non_self_seeds(&self) -> Vec<Seed> {
        self.seeds_now().iter().filter(|s| s.public_key != Some(self.public_key())).cloned().collect()
    }

    pub fn non_self_seeds_pk(&self) -> Vec<PublicKey> {
        self.seeds_now().iter().filter(|s| s.public_key != Some(self.public_key())).cloned()
            .flat_map(|s| s.public_key).collect()
    }

    pub fn secure_or(&self) -> &DataFolder {
        match &self.secure_data_folder {
            Some(folder) => folder,
            None => &self.data_folder
        }
    }

    // pub fn secure_path_or(&self) -> String {
    //     self.config_data.secure_data.
    // }


    // This should ONLY be used by the genesis node when starting for the very first time
    // Probably another way to deal with this, mostly used for debug runs and so on
    // Where seeds are being specified by CLI -- shouldn't be used by main network environments
    pub fn self_seed(&self) -> Seed {
        Seed {
            // TODO: Make this external host and attempt a DNS lookup on the seed to get the IP
            external_address: self.external_ip.clone(),
            environments: vec![self.network.clone() as i32],
            port_offset: Some(self.port_offset.clone() as u32),
            trust: vec![TrustData::from_label(1.0)],
            peer_id: Some(self.peer_id()),
            public_key: Some(self.public_key()),
        }
    }

    pub fn is_self_seed(&self) -> bool {
        self.seeds_now_pk().iter().any(|pk| pk == &self.public_key())
    }

    pub fn peer_id(&self) -> PeerId {
        self.peer_id.clone()
    }

    pub fn env_data_folder(&self) -> EnvDataFolder {
        self.data_folder.by_env(self.network)
    }

    pub fn data_store_path(&self) -> String {
        self.env_data_folder().data_store_path().to_str().unwrap().to_string()
    }

    // TODO: this can be fixed at arg parse time
    pub fn public_key(&self) -> PublicKey {
        self.public_key.clone()
    }

    pub fn short_id(&self) -> Result<String, ErrorInfo> {
        self.public_key().hex().short_string()
    }

    pub fn gauge_id(&self) -> [(String, String); 1] {
        [("public_key".to_string(), self.short_id().expect("short id"))]
    }

    pub fn version_info(&self) -> VersionInfo {
        VersionInfo{
            executable_checksum: self.executable_checksum.clone().unwrap_or("".to_string()),
            commit_hash: None,
            // TODO: Move these fields into a different struct so they can be updated
            next_upgrade_time: None,
            next_executable_checksum: None,
            build_number: Some(Self::build_number())
        }
    }

    pub fn build_number() -> i64 {
        include_str!("../resources/build_number").to_string()
            .split("\n")
            .next()
            .map(|s| s.trim())
            .and_then(|s| s.parse::<i64>().error_info(format!("Build number {s}")).log_error().ok())
            .unwrap_or(0)
    }

    pub fn node_metadata_fixed(&self) -> NodeMetadata {
        NodeMetadata{
            transport_info: Some(TransportInfo{
                external_ipv4: Some(self.external_ip.clone()),
                external_ipv6: None,
                external_host: Some(self.external_host.clone()),
                port_offset: Some(self.port_offset as i64),
                nat_restricted: None,
            }),
            public_key: Some(self.public_key()),
            node_type: Some(NodeType::Static as i32),
            version_info: Some(self.version_info()),
            partition_info: None,
            peer_id: Some(self.peer_id.clone()),
            node_name: None,
            parties: vec![],
        }
    }

    //
    pub fn dynamic_node_metadata_fixed(&self) -> DynamicNodeMetadata {
        DynamicNodeMetadata {
            udp_port: None,
            proof: None,
            peer_id: None,
            sequence: 0,
        }
    }

    pub fn is_local_debug(&self) -> bool {
        self.network == NetworkEnvironment::Local || self.network == NetworkEnvironment::Debug
    }

    pub fn is_debug(&self) -> bool {
        self.network == NetworkEnvironment::Debug
    }

    pub fn main_stage_network(&self) -> bool {
        self.network == NetworkEnvironment::Main ||
        self.network == NetworkEnvironment::Test ||
        self.network == NetworkEnvironment::Staging ||
        self.network == NetworkEnvironment::Dev ||
            self.network == NetworkEnvironment::Predev
    }

    pub fn address(&self) -> Address {
        self.public_key().address().expect("address")
    }


    pub fn control_port(&self) -> u16 {
        self.control_port.unwrap_or(self.port_offset - 10)
    }
    pub fn p2p_port(&self) -> u16 {
        self.p2p_port.unwrap_or(self.port_offset + 0)
    }

    pub fn public_port(&self) -> u16 {
        self.public_port.unwrap_or(self.port_offset + 1)
    }

    pub fn placeholder_port(&self) -> u16 {
        self.port_offset + 2
    }


    pub fn rosetta_port(&self) -> u16 {
        self.rosetta_port.unwrap_or(self.port_offset + 3)
    }

    pub fn mparty_port(&self) -> u16 {
        self.port_offset + 4
    }

    pub fn udp_port(&self) -> u16 {
        self.port_offset + 5
    }

    pub fn explorer_port(&self) -> u16 {
        self.port_offset + 6
    }

    pub fn default() -> Self {
        Self {
            config_data: Default::default(),
            peer_id: Default::default(),
            public_key: structs::PublicKey::default(),
            mnemonic_words: "".to_string(),
            port_offset: NetworkEnvironment::Debug.default_port_offset(),
            p2p_port: None,
            control_port: None,
            public_port: None,
            rosetta_port: None,
            disable_control_api: false,
            disable_public_api: false,
            seed_hosts: vec![],
            observation_formation_millis: Duration::from_millis(OBSERVATION_FORMATION_TIME_MILLIS),
            transaction_finalization_time: Duration::from_millis(
                STANDARD_FINALIZATION_INTERVAL_MILLIS,
            ),
            reward_poll_interval_secs: REWARD_POLL_INTERVAL,
            network: NetworkEnvironment::Debug,
            check_observations_done_poll_interval: Duration::from_secs(1),
            check_observations_done_poll_attempts: 3,
            seeds: vec![],
            manually_added_seeds: vec![],
            ignore_default_seeds: false,
            executable_checksum: None,
            disable_auto_update: false,
            auto_update_poll_interval: Duration::from_secs(60),
            block_formation_interval: Duration::from_secs(10),
            genesis_config: GenesisConfig{
            },
            faucet_enabled: true,
            e2e_enabled: false,
            load_balancer_url: "lb.redgold.io".to_string(),
            external_ip: "127.0.0.1".to_string(),
            external_host: "localhost".to_string(),
            servers: vec![],
            log_level: "DEBUG".to_string(),
            data_folder: DataFolder::target(0),
            secure_data_folder: None,
            enable_logging: true,
            discovery_interval: Duration::from_secs(5),
            shuffle_interval: Duration::from_secs(600),
            live_e2e_interval: Duration::from_secs(60*10), // every 10 minutes
            genesis: false,
            opts: Arc::new(empty_args()),
            mempool: Default::default(),
            tx_config: Default::default(),
            observation: Default::default(),
            node_info: NodeInfoConfig::default(),
            contract: Default::default(),
            contention: Default::default(),
            default_timeout: Duration::from_secs(150),
            disable_metrics: false,
            args: Arc::new(vec![]),
            abort: false,
            is_gui: false,
        }
    }

    pub fn memdb_path(seed_id: &u16) -> String {
        "file:memdb1_id".to_owned() + &*seed_id.clone().to_string() + "?mode=memory&cache=shared"
    }

    pub fn secure_path(&self) -> Option<String> {
        // TODO: Move to arg translate
        std::env::var("REDGOLD_SECURE_DATA_PATH").ok()
    }

    // TODO: this is wrong
    pub fn secure_all_path(&self) -> Option<String> {
        // TODO: Move to arg translate
        std::env::var("REDGOLD_SECURE_DATA_PATH").ok().map(|p| {
            let buf = PathBuf::from(p);
            buf.join(NetworkEnvironment::All.to_std_string())
        }).map(|p| p.to_str().expect("failed to render ds path").to_string())
    }

    pub fn secure_mnemonic(&self) -> Option<String> {
        self.secure_all_path().and_then(|p| {
            fs::read_to_string(p).ok()
        })
    }

}
