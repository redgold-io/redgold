use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::{RgResult, SafeOption};
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::structs::{Address, CurrencyAmount, ExternalTransactionId};
use crate::monero::key_derive::MoneroSeedBytes;
use crate::monero::rpc_core::MoneroRpcWrapper;
use crate::monero::rpc_multisig::{ExchangeMultisigKeysResult, MakeMultisigResult};
use crate::word_pass_support::WordsPassNodeConfig;

/// This is the main interface the internal node process uses to interact with the Monero network.
/// It should primarily and only be used for multi-sig operations
/// It is not thread-safe, interactions with the docker process running the wallet daemon
/// should be done in a single-threaded manner.
/// More importantly, each interaction updates this interface's internal state (representing the
/// wallets external state)
#[derive(Clone)]
pub struct MoneroNodeRpcInterfaceWrapper {
    pub rpc: MoneroRpcWrapper,
    pub state : MoneroWalletMultisigRpcState,
    pub nc: NodeConfig
}

#[derive(Clone)]
pub enum MoneroWalletMultisigRpcState {
    Unknown,
    Prepared(String),
    Made(MakeMultisigResult),
    Exchanged(ExchangeMultisigKeysResult),
    Finalized(String),
    MultisigReadyToSend
}

impl MoneroWalletMultisigRpcState {
    pub fn multisig_address_str(&self) -> Option<String> {
        match self {
            MoneroWalletMultisigRpcState::Unknown => None,
            MoneroWalletMultisigRpcState::Prepared(_) => None,
            MoneroWalletMultisigRpcState::Made(m) => Some(m.address.clone()),
            MoneroWalletMultisigRpcState::Exchanged(e) => Some(e.address.clone()),
            MoneroWalletMultisigRpcState::Finalized(f) => Some(f.clone()),
            MoneroWalletMultisigRpcState::MultisigReadyToSend => {None}
        }
    }
    pub fn multisig_typed_address_external(&self) -> Option<Address> {
        // TODO: is this right address? internal or external?
        self.multisig_address_str().map(|s| Address::from_monero_external(&s))
    }

    pub fn multisig_info_string(&self) -> Option<String> {
        match self {
            MoneroWalletMultisigRpcState::Unknown => {None}
            MoneroWalletMultisigRpcState::Prepared(s) => {Some(s.clone())}
            MoneroWalletMultisigRpcState::Made(m) => {Some(m.multisig_info.clone())}
            MoneroWalletMultisigRpcState::Exchanged(e) => {Some(e.multisig_info.clone())}
            MoneroWalletMultisigRpcState::Finalized(e) => { None }
            _ => { None}
        }
    }

}

impl MoneroNodeRpcInterfaceWrapper {

    pub fn from_config(nc: &NodeConfig) -> Option<RgResult<Self>> {
        let option = MoneroRpcWrapper::from_config(nc);
        option.map(|rpc| rpc.map(|r|
            Self { rpc: r, state: MoneroWalletMultisigRpcState::Unknown, nc: nc.clone() }))
    }

    pub fn self_address(&self) -> RgResult<Address> {
        self.nc.words().monero_external_address(&self.nc.network)
    }

    pub fn self_address_str(&self) -> RgResult<String> {
        self.self_address().and_then(|a| a.render_string())
    }

    pub fn view_key(&self) -> RgResult<String> {
        self.nc.words().derive_monero_keys().map(|kp| kp.view.to_string())
    }

    pub async fn set_as_multisig_self(&mut self) -> RgResult<()> {
        self.rpc.register_dupe_ok(
            self.view_key()?,
            self.self_address_str()?,
            None,
            None,
            Self::multisig_filename_prefix()
        ).await?;
        self.rpc.activate_wallet(self.self_address_str()?, Self::multisig_filename_prefix()).await?;
        Ok(())
    }

    fn multisig_filename_prefix() -> Option<String> {
        Some("multisig".to_string())
    }


    /*
    The correct sequence for multisig wallet setup would now be:

    prepare_multisig()
    make_multisig()
    exchange_multisig_keys()
    finalize_multisig() (for N-1/N wallets)

    Then for transactions:

    export_multisig_info() and import_multisig_info() to sync wallet states
    Create the transaction using regular wallet RPC methods
    sign_multisig() to sign the transaction
    submit_multisig() to broadcast it
     */
    pub async fn start_multisig_creation(&mut self) -> RgResult<String> {
        self.state = MoneroWalletMultisigRpcState::Unknown;
        self.rpc.close_wallet().await.log_error().ok();
        self.set_as_multisig_self().await?;
        let mut m = self.rpc.get_multisig()?;
        let prepare_result = m.prepare_multisig().await?;
        self.state = MoneroWalletMultisigRpcState::Prepared(prepare_result.clone());
        Ok(prepare_result)
    }

    pub async fn multisig_create_next(&mut self, peer_strings: Option<Vec<String>>, threshold: Option<i64>) -> RgResult<MoneroWalletMultisigRpcState> {
        match self.state.clone() {
            MoneroWalletMultisigRpcState::Unknown => {
                self.start_multisig_creation().await?;
            }
            MoneroWalletMultisigRpcState::Prepared(_) => {
                self.make_multisig(
                    peer_strings.ok_msg("missing peer strings in make multisig")?,
                    threshold.ok_msg("Missing threshold on make multisig")?
                ).await?;
            }
            MoneroWalletMultisigRpcState::Made(m) => {
                self.exchange_multisig_keys(
                    peer_strings.ok_msg("missing peer strings in exchange multisig")?
                ).await?;
            }
            MoneroWalletMultisigRpcState::Exchanged(e) => {
                self.finalize_multisig(
                    peer_strings.ok_msg("missing peer strings in finalize multisig")?
                ).await?;
            }
            MoneroWalletMultisigRpcState::Finalized(a) => {
                self.state = MoneroWalletMultisigRpcState::MultisigReadyToSend;
            }
            MoneroWalletMultisigRpcState::MultisigReadyToSend => {
                return "Multisig wallet is ready to send transactions, but create next called".to_error()
            }
        }
        Ok(self.state.clone())
    }

    pub async fn multisig_send(
        amounts: Vec<(Address, CurrencyAmount)>
    ) -> RgResult<ExternalTransactionId> {

        "Not implemented".to_error()
    }

    pub async fn make_multisig(&mut self, peer_strings: Vec<String>, threshold: i64) -> RgResult<MakeMultisigResult> {
        let mut m = self.rpc.get_multisig()?;
        let make_result = m.make_multisig(peer_strings, threshold as u32, "".to_string()).await?;
        self.state = MoneroWalletMultisigRpcState::Made(make_result.clone());
        Ok(make_result)
    }

    pub async fn exchange_multisig_keys(&mut self, peer_strings: Vec<String>) -> RgResult<ExchangeMultisigKeysResult> {
        let mut m = self.rpc.get_multisig()?;
        let exchanged = m.exchange_multisig_keys(peer_strings, "".to_string(), None).await?;
        self.state = MoneroWalletMultisigRpcState::Exchanged(exchanged.clone());
        Ok(exchanged)
    }
    pub async fn finalize_multisig(&mut self, infos: Vec<String>) -> RgResult<String> {
        let mut m = self.rpc.get_multisig()?;
        let finalized = m.finalize_multisig(infos, "".to_string()).await?;
        self.state = MoneroWalletMultisigRpcState::Finalized(finalized.clone());
        Ok(finalized)
    }

}


//#[ignore]
#[tokio::test]
async fn local_three_node() {
    let one = NodeConfig::from_test_id(&(0 as u16));
    let two = NodeConfig::from_test_id(&(1 as u16));
    let three = NodeConfig::from_test_id(&(2 as u16));
}