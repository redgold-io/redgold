use crate::conf::rg_args::RgTopLevelSubcommand;
use crate::config_data::{ConfigData, RpcUrl};
use crate::constants::{OBSERVATION_FORMATION_TIME_MILLIS, REWARD_POLL_INTERVAL, STANDARD_FINALIZATION_INTERVAL_MILLIS};
use crate::data_folder::{DataFolder, EnvDataFolder};
use crate::keys::words_pass::WordsPass;
use crate::observability::errors::Loggable;
use crate::proto_serde::ProtoSerde;
use crate::seeds::get_seeds_by_env_time;
use crate::servers::ServerOldFormat;
use crate::structs::{Address, DynamicNodeMetadata, ErrorInfo, NetworkEnvironment, NodeMetadata, NodeType, PeerId, PublicKey, Seed, SupportedCurrency, TransportInfo, TrustData, VersionInfo};
use crate::util::times::current_time_millis;
use crate::{structs, ErrorInfoContext, RgResult, SafeOption, ShortString};
use itertools::Itertools;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

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

// TODO: put the default node configs here
#[derive(Clone, Debug)]
pub struct NodeConfig {
    pub config_data: Arc<ConfigData>,
    pub peer_id: PeerId,
    pub public_key: PublicKey,
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
    pub load_balancer_url: String,
    pub external_ip: String,
    pub external_host: String,
    pub log_level: String,
    pub data_folder: DataFolder,
    pub secure_data_folder: Option<DataFolder>,
    pub enable_logging: bool,
    pub discovery_interval: Duration,
    pub shuffle_interval: Duration,
    pub mempool: MempoolConfig,
    pub tx_config: TransactionProcessingConfig,
    pub observation: ObservationConfig,
    pub contract: ContractConfig,
    pub contention: ContentionConfig,
    pub default_timeout: Duration,
    pub disable_metrics: bool,
    pub args: Arc<Vec<String>>,
    pub abort: bool,
    pub is_gui: bool,
    pub top_level_subcommand: Option<Box<RgTopLevelSubcommand>>
}



impl NodeConfig {

    // pub fn cli_get_words_pass(&self) -> WordsPass {
    //     self.config_data.cli.as_ref().and_then(|c| c.
    // }

    pub fn allowed_proxy_origins(&self) -> Vec<String> {
        self.config_data.node.as_ref().and_then(|n| n.allowed_http_proxy_origins.clone()).unwrap_or(vec![])
    }


    pub fn websocket_rpcs(&self, cur: SupportedCurrency) -> Vec<String> {
        self.config_data.external.as_ref()
            .and_then(|e| e.rpcs.as_ref())
            .map(|r| r.iter()
                .filter(|r| r.currency == cur)
                .filter(|r| r.url.starts_with("ws"))
                .map(|r| r.url.clone()).collect::<Vec<String>>()
            ).unwrap_or_default()
    }

}

impl NodeConfig {

    pub fn set_words(&mut self, words: String) {
        let mut data = (*self.config_data).clone();
        data.node.get_or_insert(Default::default()).words = Some(words);
        self.config_data = Arc::new(data);
    }

    pub fn set_rpcs(&mut self, rpcs: Vec<RpcUrl>) {
        let mut data = (*self.config_data).clone();
        data.external.get_or_insert(Default::default()).rpcs = Some(rpcs);
        self.config_data = Arc::new(data);
    }

    pub fn mnemonic_words(&self) -> String {
        self.config_data.node.as_ref().and_then(|n| n.words.clone()).expect("mnemonic words")
    }

    pub fn secure_mnemonic_words(&self) -> Option<String> {
        self.config_data.secure.as_ref().and_then(|n| n.salt.clone())
    }

    pub fn secure_mnemonic_words_or(&self) -> String {
        self.secure_mnemonic_words().unwrap_or(self.mnemonic_words())
    }

    pub fn e2e_enabled(&self) -> bool {
        self.config_data.debug.as_ref().and_then(|d| d.enable_live_e2e).unwrap_or(false)
    }
    pub fn genesis(&self) -> bool {
        self.config_data.debug.as_ref().and_then(|d| d.genesis).unwrap_or(false)
    }

    pub fn args(&self) -> Vec<&String> {
        self.args.iter().dropping(1).collect()
    }

    pub fn arg_at(&self, index: impl Into<i32>) -> RgResult<&String> {
        self.args().get(index.into() as usize).ok_msg("arg not found").cloned()
    }

    pub fn use_e2e_external_resource_mocks(&self) -> bool {
        self.config_data.debug.as_ref().and_then(|d| d.use_e2e_external_resource_mocks).unwrap_or(false)
    }

    pub fn order_cutoff_delay_time(&self) -> Duration {
        let option = self.config_data.party.as_ref()
            .and_then(|p| p.order_cutoff_delay_time)
            .unwrap_or(300_000i64);
        Duration::from_millis(option as u64)
    }

    pub fn poll_interval(&self) -> Duration {
        let option = self.config_data.party.as_ref()
            .and_then(|p| p.poll_interval)
            .unwrap_or(300_000i64);
        Duration::from_millis(option as u64)
    }

    pub fn rpc_url(&self, cur: SupportedCurrency) -> Vec<RpcUrl> {
        let mut res = vec![];
        if let Some(external) = self.config_data.external.as_ref() {
            if let Some(r) = external.rpcs.as_ref() {
                for rr in r.iter() {
                    if let Some(n) = NetworkEnvironment::from_std_string(&rr.network).ok() {
                        if rr.currency == cur && self.network == n {
                            res.push(rr.clone());
                        }
                    }
                }
            }
        }
        res
    }

    pub fn enable_party_mode(&self) -> bool {
        self.config_data.party.as_ref().and_then(|p| p.enable).unwrap_or(false)
    }

    pub fn from_email(&self) -> Option<String> {
        self.config_data.email.as_ref().and_then(|n| n.from.clone())
    }

    pub fn to_email(&self) -> Option<String> {
        self.config_data.email.as_ref().and_then(|n| n.to.clone())
    }
    pub fn multiparty_gg20_timeout(&self) -> Duration {
        Duration::from_secs(
            self.config_data.party.as_ref()
                .and_then(|n| n.gg20_peer_timeout_seconds)
                .unwrap_or(100) as u64
        )
    }

    pub fn debug_id(&self) -> Option<i32> {
        self.config_data.debug.as_ref().and_then(|d| d.id)
    }

    pub fn development_mode(&self) -> bool {
        self.config_data.debug.as_ref().and_then(|d| d.develop).unwrap_or(false)
    }

    pub fn development_mode_main(&self) -> bool {
        self.config_data.debug.as_ref().and_then(|d| d.developer).unwrap_or(false)
    }

    pub fn aws_access(&self) -> Option<String> {
        self.config_data.keys.as_ref().and_then(|k| k.aws_access.clone())
    }

    pub fn aws_secret(&self) -> Option<String> {
        self.config_data.keys.as_ref().and_then(|k| k.aws_secret.clone())
    }

    pub fn multiparty_timeout(&self) -> Duration {
        Duration::from_secs(
            self.config_data.party.as_ref()
                .and_then(|n| n.peer_timeout_seconds)
                .unwrap_or(200) as u64
        )
    }

    pub fn usb_paths_exist(&self) -> Vec<String> {
        let mut res = vec![];
        for path in self.config_data.secure.as_ref().and_then(|s| s.usb_paths.as_ref()).unwrap_or(&vec![]) {
            if PathBuf::from(path).exists() {
                res.push(path.clone());
            }
        }
        res
    }

    pub fn party_poll_interval(&self) -> Duration {
        let option = self.config_data.party.as_ref().and_then(|n| n.poll_interval);
        let opt = option
            .unwrap_or(300_000i64);
        Duration::from_millis(
             opt as u64
        )
    }

    pub fn nat_traversal_required(&self) -> bool {
        self.config_data.node.as_ref()
            .and_then(|x| x.nat_traversal_required)
            .unwrap_or(false)
    }

    pub fn udp_keepalive(&self) -> Duration {
        self.config_data.node.as_ref()
            .and_then(|x| x.udp_keepalive_seconds)
            .map(|x| Duration::from_secs(x))
            .unwrap_or(Duration::from_secs(60))
    }

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

    pub fn servers_old(&self) -> Vec<ServerOldFormat> {
        self.config_data.local.as_ref().and_then(|l| l.deploy.as_ref())
            .map(|d| d.as_old_servers())
            .unwrap_or_default()
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
            load_balancer_url: "lb.redgold.io".to_string(),
            external_ip: "127.0.0.1".to_string(),
            external_host: "localhost".to_string(),
            log_level: "DEBUG".to_string(),
            data_folder: DataFolder::target(0),
            secure_data_folder: None,
            enable_logging: true,
            discovery_interval: Duration::from_secs(5),
            shuffle_interval: Duration::from_secs(600),
            mempool: Default::default(),
            tx_config: Default::default(),
            observation: Default::default(),
            contract: Default::default(),
            contention: Default::default(),
            default_timeout: Duration::from_secs(150),
            disable_metrics: false,
            args: Arc::new(vec![]),
            abort: false,
            is_gui: false,
            top_level_subcommand: None,
        }
    }

    pub fn live_e2e_interval(&self) -> Duration {
        let t = self.config_data.debug.as_ref().and_then(|d| d.live_e2e_interval_seconds)
            .unwrap_or(60 * 10);
        Duration::from_secs(t as u64)
    }

    pub fn memdb_path(seed_id: &u16) -> String {
        "file:memdb1_id".to_owned() + &*seed_id.clone().to_string() + "?mode=memory&cache=shared"
    }

    pub fn secure_path(&self) -> Option<String> {
        self.config_data.secure.as_ref().and_then(|s| s.path.clone())
    }


}
