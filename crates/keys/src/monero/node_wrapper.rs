use std::path::PathBuf;
use crate::monero::rpc_core::MoneroRpcWrapper;
use crate::monero::rpc_multisig::{ExchangeMultisigKeysResult, MakeMultisigResult};
use crate::word_pass_support::WordsPassNodeConfig;
use redgold_common_no_wasm::ssh_like::{LocalSSHLike, SSHOrCommandLike, SSHProcessInvoke};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{Address, CurrencyAmount, ErrorInfo, ExternalTransactionId, MultipartyIdentifier, NetworkEnvironment, RoomId, SupportedCurrency, Weighting};
use redgold_schema::util::lang_util::{AnyPrinter};
use redgold_schema::{RgResult, SafeOption, ShortString};
use serde::{Deserialize, Serialize};
use redgold_schema::config_data::RpcUrl;
use crate::monero::to_address::ToMoneroAddress;
use crate::TestConstants;
use crate::util::mnemonic_support::MnemonicSupport;

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
    pub cmd: S,
    pub wallet_dir: String,
    pub wallet_exp_path: String,
    pub allow_deletes: bool,
    pub create_states: Vec<MoneroWalletMultisigRpcState>,
    pub history: Vec<StateHistoryItem>,
}

#[derive(Clone)]
pub struct StateHistoryItem {
    pub input_state: MoneroWalletMultisigRpcState,
    pub output_state: MoneroWalletMultisigRpcState,
    pub input_peer_strings: Option<Vec<String>>,
    pub input_threshold: Option<i64>,
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

    pub fn from_config_local(
        nc: &NodeConfig,
        wallet_dir: impl Into<String>,
        expect_path: impl Into<String>,
        allow_deletes: Option<bool>,
    ) -> Option<RgResult<MoneroNodeRpcInterfaceWrapper<LocalSSHLike>>> {
        let local_ssh_like = LocalSSHLike::new(None);
        MoneroNodeRpcInterfaceWrapper::<LocalSSHLike>::from_config(nc, local_ssh_like, wallet_dir, expect_path, allow_deletes)
    }

    pub fn from_config(
        nc: &NodeConfig,
        cmd: S,
        wallet_dir: impl Into<String>,
        exp_path: impl Into<String>,
        allow_deletes: Option<bool>,
    ) -> Option<RgResult<MoneroNodeRpcInterfaceWrapper<S>>> {
        let allow_deletes = allow_deletes.unwrap_or(false);
        let wallet_dir = wallet_dir.into();
        // daemon
        let daemon_opt = MoneroRpcWrapper::from_config(nc);
        // wallet
        let wallet_opt = MoneroRpcWrapper::authed_from_config(nc);
        let exp = exp_path.into();
        Self::from_daemons(cmd, allow_deletes, wallet_dir, exp, daemon_opt, wallet_opt)
    }

    fn from_daemons(
        cmd: S,
        allow_deletes: bool,
        wallet_dir: String,
        wallet_exp_path: String,
        daemon_opt: Option<RgResult<MoneroRpcWrapper>>,
        wallet_opt: Option<RgResult<MoneroRpcWrapper>>
    ) -> Option<Result<MoneroNodeRpcInterfaceWrapper<S>, ErrorInfo>> {
        daemon_opt.and_then(
            |daemon_result|
                wallet_opt.map(|wallet_result|
                    wallet_result.and_then(|wallet| daemon_result.map(|daemon|
                        MoneroNodeRpcInterfaceWrapper::<S> {
                            wallet_rpc: wallet,
                            daemon_rpc: daemon,
                            state: MoneroWalletMultisigRpcState::Unknown,
                            cmd,
                            wallet_dir,
                            wallet_exp_path,
                            allow_deletes,
                            create_states: vec![],
                            history: vec![],
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


    pub async fn restore_from_history(&mut self, h: Vec<StateHistoryItem>, wallet_filename: &String) -> RgResult<()> {
        self.prepare_wallet_fnm_and_set_multisig(wallet_filename).await?;
        let mut m = self.wallet_rpc.get_multisig()?;
        let is_ms = m.is_multisig().await?;
        if is_ms.multisig {
            return "Wallet is already multisig, did not reset properly".to_error();
        }
        for item in h.iter() {
            self.multisig_create_next(item.input_peer_strings.clone(), item.input_threshold.clone(), wallet_filename).await?;
        }
        Ok(())
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
        self.prepare_wallet_fnm_and_set_multisig(wallet_filename).await?;

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

    async fn prepare_wallet_fnm_and_set_multisig(&mut self, wallet_filename: &String) -> Result<(), ErrorInfo> {
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
        self.cmd.execute(format!("export filename={}; expect {}", f, self.wallet_exp_path.clone()), None).await?.print();
        println!("Worked!");
        self.wallet_rpc.open_wallet_filename(wallet_filename.clone()).await?;
        Ok(())
    }

    pub async fn multisig_create_next(
        &mut self,
        peer_strings: Option<Vec<String>>,
        threshold: Option<i64>,
        wallet_filename: &String
    ) -> RgResult<MoneroWalletMultisigRpcState> {

        let mut history_item = StateHistoryItem {
            input_state: self.state.clone(),
            output_state: MoneroWalletMultisigRpcState::Unknown,
            input_peer_strings: peer_strings.clone(),
            input_threshold: threshold.clone(),
        };
        match self.state.clone() {
            MoneroWalletMultisigRpcState::Unknown => {
                println!("starting");
                self.start_multisig_creation(&wallet_filename).await?;
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
        history_item.output_state = self.state.clone();
        self.history.push(history_item);
        Ok(self.state.clone())
    }

    fn id_to_filename(id: &MultipartyIdentifier) -> String {
        let string = id.proto_serialize_hex().last_n(12).unwrap();
        format!("{}_{}", Self::multisig_filename_prefix().unwrap(), string)
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


pub fn rpcs(seed_id: i64) -> Vec<RpcUrl> {
    let password = std::env::var("MONERO_TEST_RPC_PASSWORD").unwrap();
    vec![
        RpcUrl{
            currency: SupportedCurrency::Monero,
            url: format!("http://server:28{}88", seed_id),
            network: NetworkEnvironment::Main.to_std_string(),
            wallet_only: Some(true),
            authentication: Some(format!("username:{}", password)),
            file_path: None,
            ws_only: None,
            ssh_host: None,
        },
        RpcUrl{
            currency: SupportedCurrency::Monero,
            url: "http://server:18089".to_string(),
            network: NetworkEnvironment::Main.to_std_string(),
            wallet_only: Some(false),
            authentication: None,
            file_path: None,
            ws_only: None,
            ssh_host: Some("server".to_string()),
        },
    ]

}

#[ignore]
#[tokio::test]
async fn local_three_node() {

    let ci = TestConstants::test_words_pass().unwrap();
    let ci1 = ci.hash_derive_words("1").unwrap();
    let ci2 = ci.hash_derive_words("2").unwrap();
    let path = TestConstants::dev_ci_kp_path();
    let pkh = ci.private_at(path.clone()).unwrap();
    let pkh1 = ci1.private_at(path.clone()).unwrap();
    let pkh2 = ci2.private_at(path.clone()).unwrap();
    let net = NetworkEnvironment::Main;
    // let addr = ci.public_at(path.clone()).unwrap().to_monero_address_from_monero_public_format(&net).unwrap();
    // let addr1 = ci1.public_at(path.clone()).unwrap().to_monero_address_from_monero_public_format(&net).unwrap();
    // let addr2 = ci2.public_at(path.clone()).unwrap().to_monero_address_from_monero_public_format(&net).unwrap();

    // temp testing only
    let mut s = SSHProcessInvoke::new("server", None);
    let user = std::env::var("USER").unwrap();
    s.user = Some(user.clone());

    let mut one = NodeConfig::from_test_id(&(1 as u16));
    let mut two = NodeConfig::from_test_id(&(2 as u16));
    let mut three = NodeConfig::from_test_id(&(3 as u16));
    let mut four = NodeConfig::from_test_id(&(4 as u16));

    one.set_words(ci.words.clone());
    two.set_words(ci1.words.clone());
    three.set_words(ci2.words.clone());
    // let words_25 = std::env::var("MONERO_HOT_SEED").unwrap();
    // let words_24 = words_25.split(" ").take(24).collect::<Vec<&str>>().join(" ");
    four.set_words(ci.words.clone());
    // four.set_words(words_24);
    one.network = NetworkEnvironment::Main;
    two.network = NetworkEnvironment::Main;
    three.network = NetworkEnvironment::Main;
    four.network = NetworkEnvironment::Main;

    one.set_rpcs(rpcs(1));
    two.set_rpcs(rpcs(2));
    three.set_rpcs(rpcs(3));
    four.set_rpcs(rpcs(4));


    let mut one_rpc = MoneroNodeRpcInterfaceWrapper::from_config(
        &one, s.clone(), "/disk/monerotw2","~/wallet.exp".to_string(), Some(true),
    ).unwrap().unwrap();
    let mut two_rpc = MoneroNodeRpcInterfaceWrapper::from_config(
        &two, s.clone(), "/disk/monerotw3","~/wallet.exp".to_string(), Some(true)).unwrap().unwrap();
    let mut three_rpc = MoneroNodeRpcInterfaceWrapper::from_config(
        &three, s.clone(), "/disk/monerotw4","~/wallet.exp".to_string(), Some(true)).unwrap().unwrap();
    let mut four_rpc = MoneroNodeRpcInterfaceWrapper::from_config(
        &four, s.clone(), "/disk/monerow","~/wallet.exp".to_string(), Some(true)).unwrap().unwrap();
    //
    // let pub_keys = vec![
    //     one.public_key.clone(),
    //     two.public_key.clone(),
    //     three.public_key.clone()
    // ];
    // let mut mpi = MultipartyIdentifier::default();
    // mpi.party_keys = pub_keys.clone();
    // mpi.threshold = Some(Weighting::from_int_basis(2, 3));
    // mpi.room_id = Some(RoomId{
    //     uuid: Some("test".to_string()),
    // });
    //
    // let mut rpc_vecs = vec![one_rpc.clone(), two_rpc.clone(), three_rpc.clone()];
    // let mut peer_strs = vec![];
    //
    // let fnm = mpi.proto_serialize_hex().first_n(12).unwrap();
    //
    // let mut loop_vec = vec![];
    // loop {
    //     let mut new_peer_strs = vec![];
    //     let mut last_ret = MoneroWalletMultisigRpcState::Unknown;
    //     let mut i = 0;
    //     for rpc in rpc_vecs.iter_mut() {
    //         let ret = rpc.multisig_create_next(
    //             Some(peer_strs.clone()),
    //             Some(2),
    //             &fnm
    //         ).await.unwrap();
    //
    //         if i == 0 {
    //             loop_vec.push(ret.clone());
    //         }
    //         i += 1;
    //         println!("DONE wallet for peer {:?}", ret);
    //         ret.multisig_info_string().map(|ss| new_peer_strs.push(ss));
    //         if let Some(a) = ret.multisig_address_str() {
    //             if !a.is_empty() {
    //                 println!("Multisig address: {}", a);
    //             }
    //         }
    //        last_ret = ret;
    //     }
    //     peer_strs = new_peer_strs.clone();
    //     if let MoneroWalletMultisigRpcState::MultisigReadyToSend = last_ret {
    //         break;
    //     }
    // }
    //
    // for r in rpc_vecs.iter_mut() {
    //     println!("Multisig address: {:?}", r.any_multisig_addr_creation());
    // }
    //
    // let history = rpc_vecs[0].history.clone();
    //
    // let mut one_rpc_replicated = MoneroNodeRpcInterfaceWrapper::from_config(
    //     &one, s.clone(), "/disk/monerotw2","~/wallet.exp".to_string(), Some(true)
    // ).unwrap().unwrap();
    //
    // one_rpc_replicated.restore_from_history(history, &"test_filename_differenet".to_string()).await.unwrap();
    //
    // println!("Restored wallet address {}", one_rpc_replicated.any_multisig_addr_creation().unwrap());
    //
    // println!("Done");
    //
    four_rpc.wallet_rpc.register_self_activate_ok(Some("hot".to_string())).await.unwrap();
    // four_rpc.wallet_rpc.sync_info()
    let sync_info = four_rpc.wallet_rpc.refresh_sync_check_wallet().await.expect("refresh");
    println!("sync info done: {:?}", sync_info);
    // let refresh = rpc.client.clone().wallet().refresh(None).await.expect("refresh");
    let b = four_rpc.wallet_rpc.get_balance().await.unwrap();
    println!("Balance: {:?}", b);
    println!("Balance: {:?}", b.to_fractional());
    println!("Address {}", four_rpc.wallet_rpc.self_address_str().unwrap());

}