use std::fs;
use std::hash::Hash;
use redgold_data::data_store::DataStore;
use crate::genesis;
use crate::schema::structs::{Block, NetworkEnvironment, Transaction};
use redgold_schema::constants::{DEBUG_FINALIZATION_INTERVAL_MILLIS, default_node_internal_derivation_path, OBSERVATION_FORMATION_TIME_MILLIS, REWARD_POLL_INTERVAL, STANDARD_FINALIZATION_INTERVAL_MILLIS};
use redgold_keys::util::mnemonic_words::MnemonicWords;
use std::path::PathBuf;
use std::time::Duration;
use itertools::Itertools;
use log::info;
use redgold_keys::transaction_support::{TransactionBuilderSupport, TransactionSupport};
use redgold_schema::servers::Server;
use redgold_schema::{RgResult, ShortString, structs};
use redgold_schema::structs::{Address, DynamicNodeMetadata, ErrorInfo, NodeMetadata, NodeType, PeerData, PeerId, Seed, TrustData, VersionInfo};
use redgold_schema::transaction_builder::TransactionBuilder;
use redgold_schema::util::merkle;
use redgold_schema::util::merkle::MerkleTree;
use redgold_keys::util::mnemonic_support::WordsPass;
use crate::api::public_api::PublicClient;
use crate::util::cli::args::RgArgs;
use crate::util::cli::commands;
use crate::util::cli::data_folder::{DataFolder, EnvDataFolder};
use crate::util::keys::ToPublicKeyFromLib;
use redgold_schema::util::lang_util::JsonCombineResult;

pub struct CanaryConfig {}

#[derive(Clone, Debug)]
pub struct GenesisConfig {
    block: Block,
}

impl Default for GenesisConfig {
    fn default() -> Self {
        Self {
            block: genesis::create_genesis_block(),
        }
    }
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
    pub servers: Vec<Server>,
    pub log_level: String,
    pub data_folder: DataFolder,
    pub secure_data_folder: Option<DataFolder>,
    pub enable_logging: bool,
    pub discovery_interval: Duration,
    pub watcher_interval: Duration,
    pub shuffle_interval: Duration,
    pub live_e2e_interval: Duration,
    pub genesis: bool,
    pub opts: RgArgs,
    pub mempool: MempoolConfig,
    pub tx_config: TransactionProcessingConfig,
    pub observation: ObservationConfig,
    pub contract: ContractConfig,
    pub contention: ContentionConfig,
    pub node_info: NodeInfoConfig
}

impl NodeConfig {


    pub fn secure_or(&self) -> &DataFolder {
        match &self.secure_data_folder {
            Some(folder) => folder,
            None => &self.data_folder
        }
    }

    pub fn default_peer_id(&self) -> RgResult<PeerId> {
        let pk = self.words().default_pid_kp().expect("").public_key();
        let pid = PeerId::from_pk(pk);
        Ok(pid)
    }

    pub fn words(&self) -> WordsPass {
        WordsPass::new(self.mnemonic_words.clone(), None)
    }

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
            peer_id: Some(self.peer_id.clone()),
            public_key: Some(self.public_key()),
        }
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
    pub fn public_key(&self) -> structs::PublicKey {
        self.words().default_kp().expect("").public_key()
    }

    pub fn short_id(&self) -> Result<String, ErrorInfo> {
        self.public_key().hex()?.short_string()
    }

    pub fn version_info(&self) -> VersionInfo {
        VersionInfo{
            executable_checksum: self.executable_checksum.clone().unwrap_or("".to_string()),
            commit_hash: None,
            next_upgrade_time: None,
            next_executable_checksum: None,
        }
    }

    pub fn node_metadata_fixed(&self) -> NodeMetadata {
        let pair = self.words().default_kp().expect("words");
        let _pk_vec = pair.public_key_vec();
        NodeMetadata{
            external_address: self.external_ip.clone(),
            public_key: Some(self.public_key()),
            node_type: Some(NodeType::Static as i32),
            version_info: Some(self.version_info()),
            partition_info: None,
            port_offset: Some(self.port_offset as i64),
            alias: None,
            name: None,
            peer_id: Some(self.peer_id.clone()),
            nat_restricted: None,
            // network_environment: self.network as i32,
            network_environment: self.network.clone() as i32,
            external_ipv4: None,
            external_ipv6: None,
            external_host: None
        }
    }

    //
    pub fn peer_tx_fixed(&self) -> Transaction {

        let pair = self.words().default_pid_kp().expect("");
        let mut pd = PeerData::default();
        pd.peer_id = Some(self.peer_id());
        pd.node_metadata = vec![self.node_metadata_fixed()];
        pd.version_info = Some(self.version_info());

        let tx = TransactionBuilder::new()
            .with_output_peer_data(&pair.address_typed(), pd, 0)
            .with_peer_genesis_input(&pair.address_typed())
            .transaction.sign(&pair).expect("Failed signing?").clone();

        let result = self.env_data_folder().peer_tx();
        info!("Peer loaded from env data folder result {:?}", result.clone().json_or_combine());
        result.unwrap_or(tx)
    }

    pub fn dynamic_node_metadata_fixed(&self) -> DynamicNodeMetadata {
        DynamicNodeMetadata {
            udp_port: None,
            proof: None,
            peer_id: None,
            height: 0,
        }
    }

    pub fn node_tx_fixed(&self, opt: Option<&NodeMetadata>) -> Transaction {
        let pair = self.words().default_kp().expect("");
        let mut tx = TransactionBuilder::new().with_output_node_metadata(
            &pair.address_typed(), opt.cloned().unwrap_or(self.node_metadata_fixed()), 0
        ).with_peer_genesis_input(&pair.address_typed())
            .transaction.clone();
        tx.sign(&pair).expect("sign")
    }

    pub fn lb_client(&self) -> PublicClient {
        let vec = self.load_balancer_url.split(":").collect_vec();
        let last = vec.get(vec.len() - 1).unwrap().to_string();
        let maybe_port = last.parse::<u16>();
        let (host, port) = match maybe_port {
            Ok(p) => {
                (vec.join(":").to_string(), p)
            },
            Err(_) => {
                (self.load_balancer_url.clone(), self.network.default_port_offset() + 1)
            }
        };
        info!("Load balancer host: {} port: {:?}", host, port);
        PublicClient::from(host, port, None)
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

    pub fn genesis_transaction(&self) -> Transaction {
        self.genesis_config
            .block
            .transactions
            .get(0)
            .expect("filled")
            .clone()
    }

    pub fn control_port(&self) -> u16 {
        self.control_port.unwrap_or(self.port_offset - 3)
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

    pub fn default_debug() -> Self {
        NodeConfig::from_test_id(&(0 as u16))
    }

    pub fn default() -> Self {
        Self {
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
            executable_checksum: None,
            disable_auto_update: false,
            auto_update_poll_interval: Duration::from_secs(60),
            block_formation_interval: Duration::from_secs(10),
            genesis_config: Default::default(),
            faucet_enabled: true,
            e2e_enabled: true,
            load_balancer_url: "lb.redgold.io".to_string(),
            external_ip: "127.0.0.1".to_string(),
            external_host: "localhost".to_string(),
            servers: vec![],
            log_level: "DEBUG".to_string(),
            data_folder: DataFolder::target(0),
            secure_data_folder: None,
            enable_logging: true,
            discovery_interval: Duration::from_secs(5),
            watcher_interval: Duration::from_secs(200),
            shuffle_interval: Duration::from_secs(600),
            live_e2e_interval: Duration::from_secs(60),
            genesis: false,
            opts: RgArgs::default(),
            mempool: Default::default(),
            tx_config: Default::default(),
            observation: Default::default(),
            node_info: NodeInfoConfig::default(),
            contract: Default::default(),
            contention: Default::default(),
        }
    }

    pub fn memdb_path(seed_id: &u16) -> String {
        "file:memdb1_id".to_owned() + &*seed_id.clone().to_string() + "?mode=memory&cache=shared"
    }

    pub fn from_test_id(seed_id: &u16) -> Self {
        let words = redgold_keys::util::mnemonic_builder::from_str_rounds(
            &*seed_id.clone().to_string(),
            0,
        )
        .to_string();
        // let path: String = ""
        let folder = DataFolder::target(seed_id.clone() as u32);
        folder.delete().ensure_exists();
        // folder.ensure_exists();
        let mut node_config = NodeConfig::default();
        node_config.mnemonic_words = words;
        node_config.peer_id = node_config.default_peer_id().expect("worx");
        node_config.port_offset = (node_config.port_offset + (seed_id.clone() * 100)) as u16;
        node_config.data_folder = folder;
        node_config.observation_formation_millis = Duration::from_millis(1000 as u64);
        node_config.transaction_finalization_time =
            Duration::from_millis(DEBUG_FINALIZATION_INTERVAL_MILLIS);
        node_config.network = NetworkEnvironment::Debug;
        node_config.check_observations_done_poll_interval = Duration::from_secs(1);
        node_config.check_observations_done_poll_attempts = 5;
        node_config.e2e_enabled = false;
        node_config
    }
    pub fn internal_mnemonic(&self) -> MnemonicWords {
        MnemonicWords::from_mnemonic_words(&*self.mnemonic_words, None)
    }

    pub async fn data_store(&self) -> DataStore {
        DataStore::from_config_path(&self.env_data_folder().data_store_path()).await
    }

    pub async fn data_store_all(&self) -> DataStore {
        let all = self.data_folder.all().data_store_path();
        DataStore::from_file_path(all.to_str().expect("failed to render ds path").to_string()).await
    }

    pub async fn data_store_all_from(top_level_folder: String) -> DataStore {
        let p = PathBuf::from(top_level_folder.clone());
        let all = p.join(NetworkEnvironment::All.to_std_string());
        DataStore::from_file_path(all.to_str().expect("failed to render ds path").to_string()).await
    }

    pub async fn data_store_all_secure(&self) -> Option<DataStore> {
        // TODO: Move to arg translate
        if let Some(sd) = std::env::var(commands::REDGOLD_SECURE_DATA_PATH).ok() {
            Some(Self::data_store_all_from(sd).await)
        } else {
            None
        }
    }

    pub fn secure_path(&self) -> Option<String> {
        // TODO: Move to arg translate
        std::env::var(commands::REDGOLD_SECURE_DATA_PATH).ok()
    }

    pub fn secure_all_path(&self) -> Option<String> {
        // TODO: Move to arg translate
        std::env::var(commands::REDGOLD_SECURE_DATA_PATH).ok().map(|p| {
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

// TODO: Update function!
pub fn peer_id_from_single_mnemonic(mnemonic_words: String) -> Result<MerkleTree, ErrorInfo> {
    let wallet = MnemonicWords::from_mnemonic_words(&*mnemonic_words, None);
    let (_, pk) = wallet.active_key();
    let h = structs::Hash::digest(pk.serialize().to_vec());
    merkle::build_root(vec![h])
}

#[test]
fn debug(){

}