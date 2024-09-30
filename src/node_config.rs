use std::hash::Hash;
use std::path::PathBuf;
use std::time::Duration;
use itertools::Itertools;
use tracing::info;
use redgold_common_no_wasm::tx_new::TransactionBuilderSupport;
use redgold_data::data_store::{DataStore, EnvDataFolderSupport};
use crate::schema::structs::{NetworkEnvironment, Transaction};
use redgold_keys::KeyPair;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_schema::structs::{NodeMetadata, PeerId, PeerMetadata};
use redgold_schema::tx::tx_builder::TransactionBuilder;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::conf::rg_args::RgArgs;
use redgold_schema::constants::DEBUG_FINALIZATION_INTERVAL_MILLIS;
use redgold_schema::data_folder::DataFolder;
use redgold_schema::RgResult;
use redgold_schema::util::lang_util::{AnyPrinter, JsonCombineResult};
use crate::api::public_api::PublicClient;
use crate::api::RgHttpClient;
use crate::util::cli::arg_parse_config::ArgTranslate;
//
// impl Default for GenesisConfig {
//     fn default() -> Self {
//         Self {
//             block: genesis::create_genesis_block(),
//         }
//     }
// }

pub trait ApiNodeConfig {
    fn api_client(&self) -> PublicClient;
    fn self_client(&self) -> PublicClient;
    fn api_rg_client(&self) -> RgHttpClient;
}

impl ApiNodeConfig for NodeConfig {

    fn api_client(&self) -> PublicClient {
        let vec = self.load_balancer_url.split(":").collect_vec();
        let last = vec.get(vec.len() - 1).unwrap().to_string();
        let maybe_port = last.parse::<u16>();
        let (host, port) = match maybe_port {
            Ok(p) => {
                (vec.get(0).unwrap().to_string(), p)
            },
            Err(_) => {
                (self.load_balancer_url.clone(), self.network.default_port_offset() + 1)
            }
        };
        // info!("Load balancer host: {} port: {:?}", host, port);
        PublicClient::from(host, port, None)
    }

    fn self_client(&self) -> PublicClient {
        let host = "127.0.0.1".to_string();
        let port = self.public_port();
        PublicClient::from(host, port, None)
    }

    fn api_rg_client(&self) -> RgHttpClient {
        self.api_client().client_wrapper()
    }

}

pub trait EnvDefaultNodeConfig {
    async fn dev_default() -> Self;
    async fn default_env(network_environment: NetworkEnvironment) -> Self;
}

pub trait NodeConfigKeyPair {

    fn keypair(&self) -> KeyPair;
}

pub trait ToTransactionBuilder {
    fn tx_builder(&self) -> TransactionBuilder;
    fn peer_tx_fixed(&self) -> Transaction;
    fn node_tx_fixed(&self, opt: Option<&NodeMetadata>) -> Transaction;
}

impl EnvDefaultNodeConfig for NodeConfig {

    async fn dev_default() -> Self {
        Self::default_env(NetworkEnvironment::Dev).await
    }

    async fn default_env(network_environment: NetworkEnvironment) -> Self {
        let mut opts = RgArgs::default();
        opts.network = Some(network_environment.to_std_string());
        let mut node_config = NodeConfig::default();
        node_config.disable_metrics = true;
        let mut arg_translate = ArgTranslate::new(&opts, &node_config.clone());
        arg_translate.translate_args().await.unwrap();
        let mut nc = arg_translate.node_config;
        nc.network = network_environment.clone();
        nc
    }

}

impl NodeConfigKeyPair for NodeConfig {
    fn keypair(&self) -> KeyPair {
        self.words().default_kp().expect("")
    }
}

impl ToTransactionBuilder for NodeConfig {
    fn tx_builder(&self) -> TransactionBuilder {
        TransactionBuilder::new(self)
    }

    fn peer_tx_fixed(&self) -> Transaction {

        let pair = self.words().default_pid_kp().expect("");
        let mut pd = PeerMetadata::default();
        pd.peer_id = Some(self.peer_id());
        pd.node_metadata = vec![self.node_metadata_fixed()];
        pd.version_info = Some(self.version_info());

        let mut builder = TransactionBuilder::new(&self);
        builder.allow_bypass_fee = true;
        let tx = builder
            .with_output_peer_data(&pair.address_typed(), pd, 0)
            .with_genesis_input(&pair.address_typed())
            .transaction.sign(&pair).expect("Failed signing?").clone();

        let result = self.env_data_folder().peer_tx();
        if !self.is_local_debug() {
            info!("Peer loaded from env data folder result {:?}", result.clone().json_or_combine());
        }
        result.unwrap_or(tx)
    }

    fn node_tx_fixed(&self, opt: Option<&NodeMetadata>) -> Transaction {
        let pair = self.words().default_kp().expect("");
        let metadata = opt.cloned().unwrap_or(self.node_metadata_fixed());
        let mut builder = TransactionBuilder::new(&self);
        builder.allow_bypass_fee = true;
        let mut tx = builder.with_output_node_metadata(
            &pair.address_typed(), metadata, 0
        ).with_genesis_input(&pair.address_typed())
            .transaction.clone();
        tx.sign(&pair).expect("sign")
    }

}

pub trait WordsPassNodeConfig {
    fn words(&self) -> WordsPass;
    fn from_test_id(seed_id: &u16) -> Self;

    fn default_debug() -> Self;

    fn default_peer_id(&self) -> RgResult<PeerId>;
}

impl WordsPassNodeConfig for NodeConfig {

    fn default_peer_id(&self) -> RgResult<PeerId> {
        let pk = self.words().default_pid_kp().expect("").public_key();
        let pid = PeerId::from_pk(pk);
        Ok(pid)
    }
    fn default_debug() -> Self {
        NodeConfig::from_test_id(&(0 as u16))
    }

    fn words(&self) -> WordsPass {
        WordsPass::new(self.mnemonic_words.clone(), None)
    }

    fn from_test_id(seed_id: &u16) -> Self {
        let words = WordsPass::from_str_hashed(seed_id.to_string()).words;
        // let path: String = ""
        let folder = DataFolder::target(seed_id.clone() as u32);
        folder.delete().ensure_exists();
        // folder.ensure_exists();
        let mut node_config = NodeConfig::default();
        node_config.mnemonic_words = words;
        node_config.peer_id = node_config.default_peer_id().expect("worx");
        node_config.public_key = node_config.keypair().public_key();
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
}

pub trait DataStoreNodeConfig {
    async fn data_store(&self) -> DataStore;
    async fn data_store_all(&self) -> DataStore;
    async fn data_store_all_from(top_level_folder: String) -> DataStore;
    async fn data_store_all_secure(&self) -> Option<DataStore>;
}

impl DataStoreNodeConfig for NodeConfig {

    async fn data_store(&self) -> DataStore {
        DataStore::from_config_path(&self.env_data_folder().data_store_path()).await
    }

    async fn data_store_all(&self) -> DataStore {
        let all = self.data_folder.all().data_store_path();
        DataStore::from_file_path(all.to_str().expect("failed to render ds path").to_string()).await
    }

    async fn data_store_all_from(top_level_folder: String) -> DataStore {
        let p = PathBuf::from(top_level_folder.clone());
        let all = p.join(NetworkEnvironment::All.to_std_string());
        DataStore::from_file_path(all.to_str().expect("failed to render ds path").to_string()).await
    }

    async fn data_store_all_secure(&self) -> Option<DataStore> {
        if let Some(df) = &self.secure_data_folder {
            Some(df.all().data_store().await)
        } else {
            None
        }
    }

}

#[tokio::test]
async fn debug(){

    let mut nc = NodeConfig::default_env(NetworkEnvironment::Local).await;
    nc.load_balancer_url = "localhost:22320".to_string();
    nc.api_client().client_wrapper().url().print();
}