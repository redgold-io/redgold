use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::data_folder::DataFolder;
use redgold_schema::RgResult;
use redgold_schema::structs::{NetworkEnvironment, PeerId, SupportedCurrency};
use std::sync::Arc;
use std::time::Duration;
use redgold_schema::config_data::RpcUrl;
use redgold_schema::constants::DEBUG_FINALIZATION_INTERVAL_MILLIS;
use redgold_schema::errors::into_error::ToErrorInfo;
use crate::KeyPair;
use crate::util::mnemonic_support::WordsPass;

pub trait WordsPassNodeConfig {
    fn words(&self) -> WordsPass;
    fn from_test_id(seed_id: &u16) -> Self;

    fn default_debug() -> Self;

    fn default_peer_id(&self) -> RgResult<PeerId>;
    fn secure_words_or(&self) -> WordsPass;
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
        WordsPass::new(self.mnemonic_words().clone(), None)
    }

    fn secure_words_or(&self) -> WordsPass {
        WordsPass::new(self.secure_mnemonic_words_or(), None)
    }



    fn from_test_id(seed_id: &u16) -> Self {
        let words = WordsPass::from_str_hashed(seed_id.to_string()).words;
        // let path: String = ""
        let folder = DataFolder::target(seed_id.clone() as u32);
        folder.delete().ensure_exists();
        // folder.ensure_exists();
        let mut node_config = NodeConfig::default();
        let mut config_data = (*node_config.config_data).clone();
        let node_data = config_data.node.get_or_insert(Default::default());
        // This is only for manual testing
        let mut ext = config_data.external.get_or_insert(Default::default());
        ext.rpcs = Some(vec![
            RpcUrl{
            currency: SupportedCurrency::Monero,
            url: format!("http://server:28{}88", seed_id),
            network: NetworkEnvironment::Debug.to_std_string(),
            wallet_only: Some(true),
            authentication: Some("username:password".to_string()),
                file_path: None,
            },
            RpcUrl{
            currency: SupportedCurrency::Monero,
            url: "http://server:28089".to_string(),
            network: NetworkEnvironment::Debug.to_std_string(),
            wallet_only: Some(false),
            authentication: None,
                file_path: None,
            },
        ]);

        node_data.words = Some(words);
        config_data.debug.get_or_insert(Default::default()).enable_live_e2e = Some(false);
        node_config.config_data = Arc::new(config_data);

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
        node_config.disable_metrics = true;
        node_config
    }
}

pub trait NodeConfigKeyPair {

    fn keypair(&self) -> KeyPair;
}

impl NodeConfigKeyPair for NodeConfig {
    fn keypair(&self) -> KeyPair {
        self.words().default_kp().expect("")
    }
}