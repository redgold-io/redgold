use clap::{Args, Parser, Subcommand};

pub fn empty_args() -> RgArgs {
    RgArgs {
        config_path: None,
        words: None,
        mnemonic_path: None,
        peer_id: None,
        peer_id_path: None,
        data_folder: None,
        // Is this the right thing to do here? Good question
        network: Some("local".to_string()),
        debug_id: None,
        disable_auto_update: false,
        subcmd: None,
        genesis: false,
        seed_address: None,
        seed_port_offset: None,
        enable_live_e2e: false,
        log_level: None,
        development_mode: false,
        development_mode_main: false,
        aws_access_key_id: None,
        aws_secret_access_key: None,
        s3_backup_bucket: None,
        server_index: None,
        etherscan_api_key: None,
        from_email: None,
        to_email: None,
        enable_party_mode: false,
    }
}

/// Welcome to Redgold CLI -- here you can run a GUI, node, or use wallet or other CLI commands.
#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct RgArgs {
    /// Load configs from a specified path instead of standard path
    #[clap(short, long)]
    pub config_path: Option<String>,
    #[clap(short, long)]
    /// A directly embedded string of mnemonic words for controlling node identity
    pub words: Option<String>,
    /// Path to file containing string of mnemonic words for controlling node identity
    #[clap(long)]
    pub mnemonic_path: Option<String>,
    /// Hex encoded peer id
    #[clap(short, long)]
    pub peer_id: Option<String>,
    /// Path to file containing hex encoded peer id
    #[clap(long)]
    pub peer_id_path: Option<String>,
    /// Path to internal top level data directory
    #[clap(long)]
    pub data_folder: Option<String>,
    /// Network environment to connect to, e.g. main or test
    #[clap(long)]
    pub network: Option<String>,
    /// DEBUG ONLY PARAMETER for local testing, automatically generates keys based on index
    #[clap(long)]
    pub debug_id: Option<i32>,
    /// Disable automatic node updates based on standard release channel
    #[clap(long)]
    pub disable_auto_update: bool,
    /// Specific subcommands for different functionalities
    #[clap(subcommand)]
    pub subcmd: Option<RgTopLevelSubcommand>,
    #[clap(long)]
    /// Used to indicate the node is starting from genesis, only used for manual network
    /// initialization
    pub genesis: bool,
    #[clap(long)]
    /// Seed network address, only used for local testing and manually connecting to a specific
    /// network
    pub seed_address: Option<String>,
    #[clap(long)]
    /// Seed network port offset, only used for local testing and manually connecting to a specific
    /// network
    pub seed_port_offset: Option<i32>,
    #[clap(long, env = "REDGOLD_LIVE_E2E_ENABLED")]
    /// Debug only option to enable an internal continuous E2E test sending transactions
    pub enable_live_e2e: bool,
    #[clap(long)]
    /// Log level for redgold logs, i.e. DEBUG, INFO, WARN, ERROR, default INFO
    pub log_level: Option<String>,
    // TODO: File logger path
    /// Use development mode defaults -- only for use by developers, sets defaults to DEV
    /// Instead of Main for network for instance.
    #[clap(long, env = "REDGOLD_DEVELOPMENT_MODE")]
    pub development_mode: bool,
    /// Only for use by main developers
    #[clap(long, env = "REDGOLD_MAIN_DEVELOPMENT_MODE")]
    pub development_mode_main: bool,
    /// Used for AWS email / backups
    #[clap(long, env = "AWS_ACCESS_KEY_ID")]
    pub aws_access_key_id: Option<String>,
    /// Used for AWS email / backups
    #[clap(long, env = "AWS_SECRET_ACCESS_KEY")]
    pub aws_secret_access_key: Option<String>,
    /// Used for AWS email / backups
    #[clap(long, env = "REDGOLD_S3_BACKUP_BUCKET")]
    pub s3_backup_bucket: Option<String>,
    /// Used for backups
    #[clap(long, env = "REDGOLD_SERVER_INDEX")]
    pub server_index: Option<String>,
    /// Price oracles
    #[clap(long, env = "ETHERSCAN_API_KEY")]
    pub etherscan_api_key: Option<String>,
    /// Alerts / watched data emails
    #[clap(long, env = "REDGOLD_FROM_EMAIL")]
    pub from_email: Option<String>,
    /// Alerts / watched data emails
    #[clap(long, env = "REDGOLD_TO_EMAIL")]
    pub to_email: Option<String>,
    /// Multiparty mode enabled
    #[clap(long, env = "REDGOLD_ENABLE_PARTY_MODE")]
    pub enable_party_mode: bool,

}

impl RgArgs {
    pub fn clear_sensitive(&self) -> Self {
        let mut c = self.clone();
        c.words = None;
        c.aws_access_key_id = None;
        c.aws_secret_access_key = None;
        c
    }
}

impl Default for RgArgs {
    fn default() -> RgArgs {
        empty_args()
    }
}

#[derive(Subcommand, Debug, Clone)]
pub enum RgTopLevelSubcommand {
    #[clap(version = "1.3", author = "Redgold")]
    GUI(GUI),
    Node(NodeCli),
    AddServer(AddServer),
    SetServersCsv(SetServersCsv),
    RemoveServer(RemoveServer),
    DebugCanary(DebugCanary),
    Deploy(Deploy),
    GenerateWords(GenerateMnemonic),
    GenerateRandomWords(GenerateRandomWords),
    Send(WalletSend),
    Address(WalletAddress),
    Query(QueryCli),
    Faucet(FaucetCli),
    Balance(BalanceCli),
    TestTransaction(TestTransactionCli),
    TestCapture(TestCaptureCli),
    TestBitcoinBalance(TestBitcoinBalanceCli),
    ConvertMetadataXpub(ConvertMetadataXpub),
    GenerateConfig(GenerateConfig),
    DebugCommand(DebugCommand)
}

/// Run a native gui client
#[derive(Args, Debug, Clone)]
pub struct GUI {}

/// Run a peer to peer node
#[derive(Args, Debug, Clone)]
pub struct NodeCli {
    /// Force enable faucet
    #[clap(long)]
    pub debug_enable_faucet: bool,
    /// E2E test interval
    #[clap(long)]
    pub live_e2e_interval: Option<u64>
}

/// Add a new server by hostname and key used
#[derive(Args, Debug, Clone)]
pub struct AddServer {
    /// SSH compatible host name, either raw IP or CNAME
    #[clap(short, long)]
    pub ssh_host: String,
    /// SSH compatible user name for login, default root
    #[clap(short, long)]
    pub user: Option<String>,
    /// Path to key pair used for ssh commands, passphrases not yet supported
    #[clap(short, long)]
    pub key_path: Option<String>,
    /// Index used for key distribution, default +1 of last known index.
    #[clap(short, long)]
    pub index: Option<i64>,
    /// Index used for peer_id distribution, default 0.
    #[clap(short, long)]
    pub peer_id_index: Option<i64>

}

/// Add a new server by hostname and key used
#[derive(Args, Debug, Clone)]
pub struct SetServersCsv {
    /// Path to csv file containing server information
    /// Header format should be as follows:
    /// host, index, peer_id_index, network_environment, username, key_path
    /// Only host is required as a field.
    #[clap(short, long)]
    pub path: String
}

/// Remove a server reference by host name
#[derive(Args, Debug, Clone)]
pub struct RemoveServer {
    /// SSH compatible host name, either raw IP or CNAME
    #[clap(long)]
    host: String
}

/// Deploy all servers -- will overwrite existing software if present
#[derive(Args, Debug, Clone, Default)]
pub struct Deploy {
    /// Purge stored data
    #[clap(short, long)]
    pub purge: bool,
    /// Go through the deployment wizard process with prompts for configuring all steps
    #[clap(short, long)]
    pub wizard: bool,
    /// Indicates this starts from genesis flow or contains a genesis node, only used for debugging
    #[clap(short, long)]
    pub genesis: bool,
    /// Purge or remove metrics / logs / ops services data
    #[clap(long)]
    pub purge_ops: bool,
    /// Only deploy or redeploy the ops services
    #[clap(long)]
    pub ops: bool,
    /// Update server index
    #[clap(long)]
    pub server_index: Option<i32>,
    /// Update server index
    #[clap(long)]
    pub server_filter: Option<String>,
    /// Exclude server index
    #[clap(long)]
    pub exclude_server_index: Option<i32>,
    #[clap(long)]
    pub skip_ops: bool,
    #[clap(long)]
    pub ask_pass: bool,
    #[clap(long)]
    pub cold: bool,
    /// Whether or not to update the remote mnemonic words
    #[clap(long)]
    pub words: bool,
    /// Whether or not to update the remote peer_id
    #[clap(long)]
    pub peer_id: bool,
    #[clap(long)]
    pub words_and_id: bool,
    #[clap(long)]
    pub dry_run: bool,
    #[clap(long)]
    pub debug_skip_start: bool,
    #[clap(long)]
    pub passphrase: bool,
    #[clap(long)]
    pub hard_coord_reset: bool,
    #[clap(long)]
    pub mixing_password: Option<String>,
    #[clap(long)]
    pub server_offline_info: Option<String>,
    #[clap(long)]
    pub skip_redgold_process: bool,
    #[clap(long)]
    pub skip_logs: bool,
    #[clap(long)]
    pub disable_apt_system_init: bool,
}

/// Send a transaction from current wallet to an address
#[derive(Args, Debug, Clone)]
pub struct WalletSend {
    #[clap(short, long)]
    pub to: String,
    #[clap(short, long)]
    pub amount: f64,
    #[clap(short, long)]
    pub from: Option<String>,

}

/// Generate an address from an existing wallet or key store
#[derive(Args, Debug, Clone)]
pub struct WalletAddress {
    /// Choose a particular offset for the key from the mnemonic (last field in path)
    #[clap(short, long)]
    pub index: Option<i64>,
    /// BIP-44 path for the key, e.g. m/44'/60'/0'/0/0
    #[clap(short, long)]
    pub path: Option<String>,
}

/// Query the network for information on a particular hash
#[derive(Args, Debug, Clone)]
pub struct QueryCli {
    #[clap(long)]
    pub hash: String,
}

/// Request funds from the faucet, returns transaction hash associated with faucet transfer.
#[derive(Args, Debug, Clone)]
pub struct FaucetCli {
    /// Address to send funds to
    #[clap(short, long)]
    pub to: String,
    /// Amount of funds to request -- default 5.0
    #[clap(short, long)]
    pub amount: Option<f64>,
}

/// Check the balance of an address
#[derive(Args, Debug, Clone)]
pub struct BalanceCli {
    /// Address to check balance of
    #[clap(short, long)]
    pub address: String
}

/// Run a test transaction from faucet (environments below mainnet) and back
/// If running this on mainnet, you will need to specify a source address / UTXO / wallet
/// Will make a round trip of transactions from origin and back to preserve funds, using
/// minimum sizes.
#[derive(Args, Debug, Clone)]
pub struct TestTransactionCli {}

/// Debug webcam capture
#[derive(Args, Debug, Clone)]
pub struct TestCaptureCli {}

#[derive(Subcommand, Debug, Clone)]
pub enum RgDebugCommand {
    #[clap(version = "1.3", author = "Redgold")]
    GrafanaPublicDeploy(GrafanaPublicDeploy)
}

#[derive(Args, Debug, Clone)]
pub struct GrafanaPublicDeploy {}

/// Debug Commands
#[derive(Args, Debug, Clone)]
pub struct DebugCommand {
    #[clap(subcommand)]
    pub subcmd: Option<RgDebugCommand>
}

/// Debug btc sync functionality
#[derive(Args, Debug, Clone)]
pub struct TestBitcoinBalanceCli {

}

/// Convert Xpub Metadata
#[derive(Args, Debug, Clone)]
pub struct ConvertMetadataXpub {
    #[clap(value_parser)]
    pub metadata_file: String,
}

/// Generate a config with all values filled in
#[derive(Args, Debug, Clone)]
pub struct GenerateConfig {
}

/// Generate a mnemonic from a password (minimum 128 bits of entropy required)
#[derive(Args, Debug, Clone)]
pub struct GenerateMnemonic {
    /// Seed generation password primarily used for cold mixing to prevent leaking passphrase from hot computer
    #[clap(short, long)]
    password: Option<String>,
    #[clap(short, long, default_value = "10000")]
    rounds: i32,
    #[clap(short, long)]
    use_random_seed: bool
}

/// Generate a mnemonic word list from random entropy
#[derive(Args, Debug, Clone)]
pub struct GenerateRandomWords {
    /// Source for hardware randomness, not required unless advanced user
    #[clap(long)]
    hardware: Option<String>,
}

/// Generate a mnemonic from a password (minimum 128 bits of entropy required)
#[derive(Args, Debug, Clone)]
pub struct DebugCanary {
    /// Print debug info
    #[clap(long)]
    pub host: Option<String>,
}