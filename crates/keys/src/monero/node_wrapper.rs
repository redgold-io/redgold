use crate::monero::rpc_core::MoneroRpcWrapper;
use crate::monero::rpc_multisig::{ExchangeMultisigKeysResult, MakeMultisigResult};
use crate::word_pass_support::WordsPassNodeConfig;
use redgold_common_no_wasm::ssh_like::{SSHOrCommandLike, SSHProcessInvoke};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{Address, CurrencyAmount, ExternalTransactionId, MultipartyIdentifier, RoomId, Weighting};
use redgold_schema::util::lang_util::{AnyPrinter, WithMaxLengthString};
use redgold_schema::{RgResult, SafeOption};
use serde::{Deserialize, Serialize};

/// This is the main interface the internal node process uses to interact with the Monero network.
/// It should primarily and only be used for multi-sig operations
/// It is not thread-safe, interactions with the docker process running the wallet daemon
/// should be done in a single-threaded manner.
/// More importantly, each interaction updates this interface's internal state (representing the
/// wallets external state)
#[derive(Clone)]
pub struct MoneroNodeRpcInterfaceWrapper<S: SSHOrCommandLike> {
    pub wallet_rpc: MoneroRpcWrapper,
    pub daemon_rpc: MoneroRpcWrapper,
    pub state : MoneroWalletMultisigRpcState,
    pub nc: NodeConfig,
    pub cmd: S,
    pub wallet_dir: String,
    pub allow_deletes: bool,
    pub create_states: Vec<MoneroWalletMultisigRpcState>
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MoneroWalletMultisigRpcState {
    Unknown,
    Prepared(String),
    Made(MakeMultisigResult),
    Exchanged(ExchangeMultisigKeysResult),
    // Finalized(String),
    MultisigReadyToSend
}

impl MoneroWalletMultisigRpcState {
    pub fn multisig_address_str(&self) -> Option<String> {
        match self {
            MoneroWalletMultisigRpcState::Unknown => None,
            MoneroWalletMultisigRpcState::Prepared(_) => None,
            MoneroWalletMultisigRpcState::Made(m) => Some(m.address.clone()),
            MoneroWalletMultisigRpcState::Exchanged(e) => Some(e.address.clone()),
            // MoneroWalletMultisigRpcState::Finalized(f) => Some(f.clone()),
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
            // MoneroWalletMultisigRpcState::Finalized(e) => { None }
            _ => { None}
        }
    }

}

impl<S: SSHOrCommandLike> MoneroNodeRpcInterfaceWrapper<S> {


    pub fn any_multisig_addr_creation(&self) -> Option<String> {
        self.create_states.iter()
            .filter_map(|s| s.multisig_address_str())
            .filter(|s| !s.is_empty())
            .next()
    }
    pub fn from_config(
        nc: &NodeConfig,
        cmd: S,
        wallet_dir: impl Into<String>,
        allow_deletes: Option<bool>,
    ) -> Option<RgResult<Self>> {
        let allow_deletes = allow_deletes.unwrap_or(false);
        let wallet_dir = wallet_dir.into();
        // daemon
        let daemon_opt = MoneroRpcWrapper::from_config(nc);
        // wallet
        let wallet_opt = MoneroRpcWrapper::authed_from_config(nc);

        daemon_opt.and_then(
            |daemon_result|
                wallet_opt.map(|wallet_result|
                    wallet_result.and_then(|wallet| daemon_result.map(|daemon|
                        Self {
                            wallet_rpc: wallet,
                            daemon_rpc: daemon,
                            state: MoneroWalletMultisigRpcState::Unknown,
                            nc: nc.clone(),
                            cmd,
                            wallet_dir,
                            allow_deletes,
                            create_states: vec![],
                        }))
                    )
        )
    }

    pub async fn set_as_multisig_self(&mut self, x: &String) -> RgResult<()> {
        self.wallet_rpc.register_self_activate_ok(Some(x.clone())).await
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
    pub async fn start_multisig_creation(&mut self, wallet_filename: &String) -> RgResult<String> {
        let p = self.wallet_dir.clone();
        self.wallet_rpc.close_wallet().await.log_error().ok();
        if self.allow_deletes {
            let string = format!("rm -rf {}/*", p);
            println!("Deleting wallet {}", string.clone());
            self.cmd.execute(string, None).await.unwrap().print();
            println!("deleted wallet");
        }
        self.state = MoneroWalletMultisigRpcState::Unknown;
        self.set_as_multisig_self(wallet_filename).await?;

        // Close wallet so we can reactivate it as multisig
        self.wallet_rpc.close_wallet().await.log_error().ok();
        let filename = wallet_filename.clone();
        let f = self.cmd.execute(format!("ls {}/{}* | grep -v \"\\.keys$\"", p, filename), None).await?;
        let f = f.replace("\n", "");
        println!("filename found on remote: {:?}", f.clone());
        // ret.multisig_info_string().map(|ss| peer_strs.push(ss));
        self.cmd.execute(format!("export filename={}; expect ~/wallet.exp", f), None).await?.print();
        println!("Worked!");
        self.wallet_rpc.open_wallet_filename(wallet_filename.clone()).await?;

        let mut m = self.wallet_rpc.get_multisig()?;
        let is_ms = m.is_multisig().await?;
        println!("Is multisig: {:?}", is_ms);
        let prepare_result = if is_ms.multisig {
            let ms_info = m.export_multisig_info().await?;
            println!("Exported multisig info: {:?}", ms_info);
            return "Wallet is already multisig".to_error();
        } else {
            m.prepare_multisig().await?
        };
        self.state = MoneroWalletMultisigRpcState::Prepared(prepare_result.clone());

        // let ret = rpc.multisig_create_next(Some(peer_strs.clone()), Some(2), &mpi).await.unwrap();
        println!("Created wallet for peer {:?}", prepare_result.clone());

        Ok(prepare_result)
    }
    pub async fn multisig_create_next(&mut self, peer_strings: Option<Vec<String>>, threshold: Option<i64>, id: &MultipartyIdentifier) -> RgResult<MoneroWalletMultisigRpcState> {

        let msig_id = Self::id_to_filename(id);

        match self.state.clone() {
            MoneroWalletMultisigRpcState::Unknown => {
                println!("starting");
                self.start_multisig_creation(&msig_id).await?;
                println!("Finished preparing")
            }
            MoneroWalletMultisigRpcState::Prepared(_) => {
                println!("making");
                self.make_multisig(
                    peer_strings.ok_msg("missing peer strings in make multisig")?,
                    threshold.ok_msg("Missing threshold on make multisig")?
                ).await?;
            }
            MoneroWalletMultisigRpcState::Made(_) => {
                self.exchange_multisig_keys(
                    peer_strings.ok_msg("missing peer strings in exchange multisig")?
                ).await?;
            }
            MoneroWalletMultisigRpcState::Exchanged(_) => {
                self.state = MoneroWalletMultisigRpcState::MultisigReadyToSend;
                // self.finalize_multisig(
                //     peer_strings.ok_msg("missing peer strings in finalize multisig")?
                // ).await?;
            }
            // MoneroWalletMultisigRpcState::Finalized(a) => {
            //     self.state = MoneroWalletMultisigRpcState::MultisigReadyToSend;
            // }
            MoneroWalletMultisigRpcState::MultisigReadyToSend => {
                return "Multisig wallet is ready to send transactions, but create next called".to_error()
            }
        }
        self.create_states.push(self.state.clone());
        Ok(self.state.clone())
    }

    fn id_to_filename(id: &MultipartyIdentifier) -> String {
        format!("{}_{}", Self::multisig_filename_prefix().unwrap(), id.proto_serialize_hex().last_n(12))
    }

    pub async fn multisig_send(
        amounts: Vec<(Address, CurrencyAmount)>
    ) -> RgResult<ExternalTransactionId> {

        "Not implemented".to_error()
    }

    pub async fn make_multisig(&mut self, peer_strings: Vec<String>, threshold: i64) -> RgResult<MakeMultisigResult> {
        let mut m = self.wallet_rpc.get_multisig()?;
        let make_result = m.make_multisig(peer_strings, threshold as u32, "".to_string()).await?;
        self.state = MoneroWalletMultisigRpcState::Made(make_result.clone());
        Ok(make_result)
    }

    pub async fn exchange_multisig_keys(&mut self, peer_strings: Vec<String>) -> RgResult<ExchangeMultisigKeysResult> {
        let mut m = self.wallet_rpc.get_multisig()?;
        let exchanged = m.exchange_multisig_keys(peer_strings, "".to_string(), None).await?;
        self.state = MoneroWalletMultisigRpcState::Exchanged(exchanged.clone());
        Ok(exchanged)
    }

    // This seems to throw an error if the exchange has already proceeded with all ?
    // pub async fn finalize_multisig(&mut self, infos: Vec<String>) -> RgResult<String> {
    //     let mut m = self.wallet_rpc.get_multisig()?;
    //     let finalized = m.finalize_multisig(infos, "".to_string()).await?;
    //     self.state = MoneroWalletMultisigRpcState::Finalized(finalized.clone());
    //     Ok(finalized)
    // }

}


#[ignore]
#[tokio::test]
async fn local_three_node() {

    // temp testing only
    let mut s = SSHProcessInvoke::new("server", None);
    let user = std::env::var("USER").unwrap();
    s.user = Some(user.clone());

    let one = NodeConfig::from_test_id(&(1 as u16));
    let two = NodeConfig::from_test_id(&(2 as u16));
    let three = NodeConfig::from_test_id(&(3 as u16));

    let mut one_rpc = MoneroNodeRpcInterfaceWrapper::from_config(
        &one, s.clone(), "/disk/monerotw2", Some(true)
    ).unwrap().unwrap();
    let mut two_rpc = MoneroNodeRpcInterfaceWrapper::from_config(
        &two, s.clone(), "/disk/monerotw3", Some(true)).unwrap().unwrap();
    let mut three_rpc = MoneroNodeRpcInterfaceWrapper::from_config(
        &three, s.clone(), "/disk/monerotw4", Some(true)).unwrap().unwrap();

    let pub_keys = vec![
        one.public_key.clone(),
        two.public_key.clone(),
        three.public_key.clone()
    ];
    let mut mpi = MultipartyIdentifier::default();
    mpi.party_keys = pub_keys.clone();
    mpi.threshold = Some(Weighting::from_int_basis(2, 3));
    mpi.room_id = Some(RoomId{
        uuid: Some("test".to_string()),
    });

    let mut rpc_vecs = vec![one_rpc.clone(), two_rpc.clone(), three_rpc.clone()];
    let mut peer_strs = vec![];

    loop {
        let mut new_peer_strs = vec![];
        let mut last_ret = MoneroWalletMultisigRpcState::Unknown;
        for rpc in rpc_vecs.iter_mut() {
            let ret = rpc.multisig_create_next(Some(peer_strs.clone()), Some(2), &mpi).await.unwrap();
            println!("DONE wallet for peer {:?}", ret);
            ret.multisig_info_string().map(|ss| new_peer_strs.push(ss));
            if let Some(a) = ret.multisig_address_str() {
                if !a.is_empty() {
                    println!("Multisig address: {}", a);
                }
            }
           last_ret = ret;
        }
        peer_strs = new_peer_strs.clone();
        if let MoneroWalletMultisigRpcState::MultisigReadyToSend = last_ret {
            break;
        }
    }

    for r in rpc_vecs.iter_mut() {
        println!("Multisig address: {:?}", r.any_multisig_addr_creation());
    }

    println!("Done");

}