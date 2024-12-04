use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};

pub fn empty_args() -> RgArgs {
    RgArgs {
        config_paths: CorePaths {
            home: None,
            config_path: None,
        },
        global_settings: GlobalSettings {
            network: Some("local".to_string()),
            log_level: None,
            offline: false,
            words: None,
            mnemonic_path: None,
        },
        cli_settings: CliSettings {
            cold: false,
            airgap: false,
            account: None,
            currency: None,
            path: None,
            verbose: false,
            quiet: false,
        },
        debug_args: DebugArgs {
            debug_id: None,
            seed_address: None,
            seed_port_offset: None,
        },
        subcmd: None,
    }
}

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct CorePaths {

    // Main loader paths for configs / data

    /// Home directory to use for default configuration and data storage, defaults to $HOME
    #[clap(long, env = "HOME")]
    pub home: Option<String>,
    /// Load configs from a specified path instead of standard path ~/.rg/config or ~/.rg/env/config
    #[clap(long, env = "REDGOLD_CONFIG")]
    pub config_path: Option<String>,

    // TODO: maybe re-add these if necessary, otherwise rely on them in config.
    // /// Default directory, relative to home, to store all data. Defaults to ~/.rg/
    // #[clap(long, env = "REDGOLD_DATA")]
    // pub data_path: Option<String>,
    // /// Bulk data directory, used for slow / non-SSD storage -- rarely required to configure
    // #[clap(long, env = "REDGOLD_BULK")]
    // pub bulk_data_path: Option<String>,
    //
    // // Secure config loaders, priority order ahead during config merge
    //
    // /// Load secure configs from a specified path instead of the standard data/config or data/env/config
    // #[clap(long, env = "REDGOLD_SECURE_CONFIG")]
    // pub secure_data_config_path: Option<String>,
    // /// Load secure data from a specified path instead of the standard data/config or data/env/config
    // #[clap(long, env = "REDGOLD_SECURE_PATH")]
    // pub secure_data_path: Option<String>,

}


#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct GlobalSettings {

    // Most important configs first:
    /// Network environment to connect to, e.g. main or test
    #[clap(long)]
    pub network: Option<String>,
    #[clap(long)]
    /// Log level for redgold logs -- for GUI or Node, i.e. DEBUG, INFO, WARN, ERROR, default INFO
    pub log_level: Option<String>,
    #[clap(long)]
    /// Disable all network requests, only use local data for offline signing or other purposes.
    pub offline: bool,

    // Below need organization.

    #[clap(short, long)]
    /// A directly embedded string of mnemonic words for controlling node identity
    pub words: Option<String>,
    /// Path to file containing string of mnemonic words for controlling node identity
    #[clap(long)]
    pub mnemonic_path: Option<String>,

}

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct CliSettings {
    /// Require CLI commands to use a cold hardware wallet
    #[clap(long, env = "REDGOLD_CLI_COLD")]
    pub cold: bool,
    /// Require CLI commands to use an airgap file output
    #[clap(long, env = "REDGOLD_CLI_AIRGAP")]
    pub airgap: bool,
    /// Optional account specifier (by name) for pre-configured key source for CLI commands
    #[clap(long, env = "REDGOLD_CLI_ACCOUNT")]
    pub account: Option<String>,
    /// Optional currency specifier (by name) for CLI commands
    #[clap(long, env = "REDGOLD_CLI_CURRENCY")]
    pub currency: Option<String>,
    /// Optional derivation path specifier for CLI commands
    #[clap(long, env = "REDGOLD_CLI_CURRENCY")]
    pub path: Option<String>,
    /// Include verbose / debug output for CLI commands instance of compact outputs.
    #[clap(long, env = "REDGOLD_CLI_VERBOSE")]
    pub verbose: bool,
    /// Remove CLI command outputs in favor of less info, ideally parse-able
    #[clap(long, env = "REDGOLD_CLI_QUIET")]
    pub quiet: bool,

}


#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct DebugArgs {
    /// DEBUG ONLY PARAMETER for local testing, automatically generates keys based on index
    #[clap(long)]
    pub debug_id: Option<i32>,
    #[clap(long)]
    /// Seed network address, only used for local testing and manually connecting to a specific
    /// network
    pub seed_address: Option<String>,
    #[clap(long)]
    /// Seed network port offset, only used for local testing and manually connecting to a specific
    /// network
    pub seed_port_offset: Option<i32>,
}

/// Welcome to Redgold CLI -- here you can run a GUI, node, or use wallet or other CLI commands.
#[derive(Parser, Debug, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct RgArgs {

    #[command(flatten)]
    pub config_paths: CorePaths,

    #[command(flatten)]
    pub global_settings: GlobalSettings,

    #[command(flatten)]
    pub cli_settings: CliSettings,

    #[command(flatten)]
    pub debug_args: DebugArgs,

    /// Specific subcommands for different functionalities
    #[clap(subcommand)]
    pub subcmd: Option<RgTopLevelSubcommand>,
    
}

impl RgArgs {
    pub fn clear_sensitive(&self) -> Self {
        let mut c = self.clone();
        c.global_settings.words = None;
        c
    }
}

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize)]
pub enum RgTopLevelSubcommand {
    #[clap(version = "1.3", author = "Redgold")]
    GUI(GUI),
    Node(NodeCli),
    // TODO: Re-enable these with new config loaders.
    // AddServer(AddServer),
    // SetServersCsv(SetServersCsv),
    // RemoveServer(RemoveServer),
    // DebugCanary(DebugCanary),
    Deploy(Deploy),
    // TODO: Re-enable this with argon2d and salt
    // GenerateWords(GenerateMnemonic),
    GenerateRandomWords(GenerateRandomWords),
    Send(WalletSend),
    Address(WalletAddress),
    Query(QueryCli),
    // This is disabled due to captcha, move it to debug commands potentially per network.
    // Faucet(FaucetCli),
    Balance(BalanceCli),
    GenerateConfig(GenerateConfig),
    DebugCommand(DebugCommand)
}

/// Run a native gui client, this is the default command if no args are supplied
/// This runs a local EGUI native interface which allows use of wallet / cold wallet /
/// deploy commands / airgap commands.
#[derive(Args, Debug, Clone, Serialize, Deserialize)]
pub struct GUI {}

/// Run a peer to peer node
#[derive(Args, Debug, Clone, Serialize, Deserialize)]
pub struct NodeCli {

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
/// Please see references to sample configuration files for setting up a deployment
/// config.toml
///
/// WARNING: The gui is actually more tested than this, but it re-uses the same code path.
/// This is still experimental through CLI.
/// You can always manually deploy as well through a direct docker compose script.
#[derive(Args, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Deploy {
    /// Purge stored data, WARNING: this will erase ALL data associated with the node, including
    /// node keys and multiparty key shares, ensure if you run this that you have them backed up.
    #[clap(short, long)]
    pub purge: bool,
    // TODO: Re-enable wizard process.
    // /// Go through the deployment wizard process with prompts for configuring all steps
    // #[clap(short, long)]
    // pub wizard: bool,
    /// Indicates this starts from genesis flow or contains a genesis node, only used for debugging
    /// or deployment of a private / local cluster. Developer only.
    #[clap(short, long)]
    pub genesis: bool,
    /// Purge or remove metrics / logs / ops services data. This is okay to run, and only removes
    /// dev information or debug information, won't remove any keys.
    #[clap(long)]
    pub purge_ops: bool,
    /// Run deployment which deploys additional operations services.
    #[clap(long)]
    pub ops: bool,
    /// Filter to only apply deployment specific to one machine, identified by its index
    #[clap(long)]
    pub server_index: Option<i32>,
    /// Filter to only apply deployment specific to one machine, identified by its name
    #[clap(long)]
    pub server_filter: Option<String>,
    /// Filter to run deployment against all servers excluding one.
    #[clap(long)]
    pub exclude_server_index: Option<i32>,
    /// Skip deploying operations (logging, metrics) services. These can be skipped to speed
    /// up redeployment in the event the dockerfile changes.
    #[clap(long)]
    pub skip_ops: bool,
    /// Require a cold deployment password input as part of the deployment mnemonic generation step
    /// Used for mixing with salt words for higher security pre-deploy.
    #[clap(long)]
    pub ask_pass: bool,
    /// Use cold info, don't generate server keys directly, instead rely on pre-signed peer
    /// transactions -- WARNING: this requires additional airgap setup steps in advance. Advanced
    /// users only
    #[clap(long)]
    pub cold: bool,
    /// Whether to update the remote mnemonic words as part of the deployment.
    #[clap(long)]
    pub words: bool,
    /// Whether to update the remote peer_idas part of the deployment.
    #[clap(long)]
    pub peer_id: bool,
    /// Whether to update both words and id as part of the deployment.
    #[clap(long)]
    pub words_and_id: bool,
    // TODO: Enable dry run to dump out commands.
    // /// Skip the actual start command, but run everything else.
    // #[clap(long)]
    // pub dry_run: bool,
    /// Skip the actual start command, but run everything else.
    #[clap(long)]
    pub debug_skip_start: bool,
    // /// Cold mixing password passed through CLI, warning, this is not secure with respect
    // /// to bash history, please know what you're doing or use the live input arg or the GUI
    // #[clap(long)]
    // pub passphrase: bool,
    /// Purge all deployment and services before attempting to restart. Useful for tearing
    /// down private/local/test network.
    #[clap(long)]
    pub hard_coord_reset: bool,
    /// Cold mixing password passed through CLI, warning, this is not secure with respect
    /// to bash history, please know what you're doing or use the live input arg or the GUI
    #[clap(long)]
    pub mixing_password: Option<String>,
    /// Path to airgap generated deployment transactions
    #[clap(long)]
    pub server_offline_info: Option<String>,
    /// Skip the redgold process, only deploy the operations services
    #[clap(long)]
    pub skip_redgold_process: bool,
    #[clap(long)]
    /// Skip logging but otherwise deploy operations.
    pub skip_logs: bool,
    /// Skip sudo / system modification commands, requires external setup in advance.
    #[clap(long)]
    pub disable_apt_system_init: bool,
}

/// Send a transaction from current (default or active) wallet to a destination address
/// expects arguments <destination> <amount>
/// Destination should be a parseable address (will waterfall parse between types.)
/// Amount should be a fractional amount, i.e. 0.1 for one tenth of a RDG
#[derive(Args, Debug, Clone, Serialize, Deserialize)]
pub struct WalletSend {
    /// Destination to send funds to, sohould be a parseable address
    pub destination: String,
    /// Amount to send, should be a fractional amount, i.e. 0.1 for one tenth of a RDG
    pub amount: f64,
}

/// Send a transaction from current (default or active) wallet to a destination address
/// expects arguments <destination> <amount>
/// Destination should be a parseable address (will waterfall parse between types.)
/// Amount should be a fractional amount, i.e. 0.1 for one tenth of a RDG
#[derive(Args, Debug, Clone)]
pub struct Swap {
    /// Optional derivation path to use for deriving the key source (for the local transaction)
    #[clap(short, long)]
    pub path: Option<String>,
    /// Currency to send, default is RDG
    #[clap(short, long)]
    pub currency: Option<String>
}

/// Generate an address from an existing wallet or key store
#[derive(Args, Debug, Clone, Serialize, Deserialize)]
pub struct WalletAddress {
    /// Choose a particular offset for the key from the mnemonic (last field in path)
    #[clap(short, long)]
    pub index: Option<i64>,
    /// BIP-44 path for the key, e.g. m/44'/60'/0'/0/0
    #[clap(short, long)]
    pub path: Option<String>,
}

/// Query the network for information on a particular hash, query <hash> as first arg
#[derive(Args, Debug, Clone, Serialize, Deserialize)]
pub struct QueryCli {
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
#[derive(Args, Debug, Clone, Serialize, Deserialize)]
pub struct BalanceCli {
    /// Address to check balance of, defaults to current active word address
    #[clap(short, long)]
    pub address: Option<String>
}

/// Run a test transaction from faucet (environments below mainnet) and back
/// If running this on mainnet, you will need to specify a source address / UTXO / wallet
/// Will make a round trip of transactions from origin and back to preserve funds, using
/// minimum sizes.
#[derive(Args, Debug, Clone)]
pub struct TestTransactionCli {}

/// Debug webcam capture
#[derive(Args, Debug, Clone)]
pub struct TestCaptureCli {
    pub cam: Option<i64>
}

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize)]
pub enum RgDebugCommand {
    #[clap(version = "1.3", author = "Redgold")]
    GrafanaPublicDeploy(GrafanaPublicDeploy),
    // TestTransaction(TestTransactionCli),
    // TestCapture(TestCaptureCli),
    // TestBitcoinBalance(TestBitcoinBalanceCli),
    // ConvertMetadataXpub(ConvertMetadataXpub),
}

#[derive(Args, Debug, Clone, Serialize, Deserialize)]
pub struct GrafanaPublicDeploy {}

/// Debug Commands
#[derive(Args, Debug, Clone, Serialize, Deserialize)]
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
#[derive(Args, Debug, Clone, Serialize, Deserialize)]
pub struct GenerateConfig {
}

/// Generate a mnemonic from a password (minimum 128 bits of entropy required)
/// Recommended to use the GUI instead of the CLI for this command, for more
/// settings.
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
#[derive(Args, Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRandomWords {
    /// Source for hardware randomness, hex encoded matching word entropy, not required
    /// unless advanced user
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