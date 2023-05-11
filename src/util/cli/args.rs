use clap::{Args, Parser, Subcommand};


pub fn empty_args() -> RgArgs {
    RgArgs {
        config_path: None,
        words: None,
        mnemonic_path: None,
        peer_id: None,
        peer_id_path: None,
        data_store_path: None,
        wallet_path: None,
        network: Some("local".to_string()),
        debug_id: None,
        disable_auto_update: false,
        subcmd: None,
        genesis: false,
        seed_address: None,
        seed_port_offset: None
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
    /// Path to internal data store, overrides default home directory path
    #[clap(long)]
    pub data_store_path: Option<String>,
    /// Path to internal wallet / key store, overrides default home directory path
    #[clap(long)]
    pub wallet_path: Option<String>,
    /// Network environment to connect to, e.g. main or test
    #[clap(long)]
    pub network: Option<String>,
    /// DEBUG ONLY PARAMETER for local testing, automatically generates keys based on index
    #[clap(long)]
    pub debug_id: Option<i32>,
    /// Enable node automatic updates based on release channel
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
    Balance(BalanceCli)
}


/// Run a native gui client
#[derive(Args, Debug, Clone)]
pub struct GUI {}

/// Run a peer to peer node
#[derive(Args, Debug, Clone)]
pub struct NodeCli {
    debug_enable_faucet: bool
}


/// Add a new server by hostname and key used
#[derive(Args, Debug, Clone)]
pub struct AddServer {
    /// SSH compatible host name, either raw IP or CNAME
    #[clap(short, long)]
    pub host: String,
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
    #[clap(short, long)]
    host: String
}

/// Deploy all servers -- will overwrite existing software if present
#[derive(Args, Debug, Clone)]
pub struct Deploy {
    /// Purge stored data
    #[clap(short, long)]
    purge: bool,
    /// Go through the deployment wizard process with prompts for configuring all steps
    #[clap(short, long)]
    pub(crate) wizard: bool,

}

// Wallet commands

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
    #[clap(short, long)]
    hardware: Option<String>,
}

/// Generate a mnemonic from a password (minimum 128 bits of entropy required)
#[derive(Args, Debug, Clone)]
pub struct DebugCanary {
    /// Print debug info
    #[clap(long)]
    pub host: Option<String>,
}
