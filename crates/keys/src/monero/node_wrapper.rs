use std::path::PathBuf;
use crate::monero::rpc_core::MoneroRpcWrapper;
use crate::monero::rpc_multisig::{ExchangeMultisigKeysResult, MakeMultisigResult};
use crate::word_pass_support::WordsPassNodeConfig;
use redgold_common_no_wasm::ssh_like::{LocalSSHLike, SSHOrCommandLike, SSHProcessInvoke};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{Address, CurrencyAmount, ErrorInfo, ExternalTransactionId, Hash, MoneroMultisigFormationRequest, MultipartyIdentifier, NetworkEnvironment, PublicKey, RoomId, SupportedCurrency, Weighting};
use redgold_schema::message::Request;
use redgold_schema::util::lang_util::{AnyPrinter};
use redgold_schema::{RgResult, SafeOption, ShortString};
use serde::{Deserialize, Serialize};
use redgold_common::external_resources::PeerBroadcast;
use redgold_schema::config_data::RpcUrl;
use crate::monero::to_address::ToMoneroAddress;
use crate::TestConstants;
use crate::util::mnemonic_support::MnemonicSupport;

use super::rpc_multisig::{DescribeTransferResult, SignedMultisigTxset, TransferSplitResult};

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


#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
pub struct PartySecretInstanceData {
    pub address: Address,
    pub monero_history: Option<Vec<StateHistoryItem>>,
}


#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
pub struct PartySecretData {
    pub instances: Vec<PartySecretInstanceData>
}


#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
pub struct StateHistoryItem {
    pub input_state: MoneroWalletMultisigRpcState,
    pub output_state: MoneroWalletMultisigRpcState,
    pub input_peer_strings: Option<Vec<String>>,
    pub input_threshold: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
pub enum MoneroWalletMultisigRpcState {
    #[default]
    Unknown,
    Prepared(String),
    Made(MakeMultisigResult),
    Exchanged(ExchangeMultisigKeysResult),
    // Finalized(String),
    MultisigReadyToSend
}

impl MoneroWalletMultisigRpcState {

    pub fn is_before_final_state(&self) -> bool {
        match &self {
            MoneroWalletMultisigRpcState::Made(_) => true,
            _ => false
        }
    }
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


    pub fn reset(&mut self) {
        self.create_states = vec![];
        self.history = vec![];
        self.state = MoneroWalletMultisigRpcState::Unknown;
    }

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

    pub fn get_secret(&self) -> RgResult<PartySecretInstanceData> {
        Ok(PartySecretInstanceData {
            address: Address::from_monero_external(&self.any_multisig_addr_creation().ok_msg("No multisig address found")?),
            monero_history: Some(self.history.clone()),
        })
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


    pub async fn multisig_create_loop<B>(
        &mut self,
        all_pks: &Vec<PublicKey>,
        threshold: i64,
        peer_broadcast: &B
    ) -> RgResult<PartySecretInstanceData> where B: PeerBroadcast {
        let b = Self::get_wallet_filename_id(all_pks, threshold);
        let wallet_filename = b;
        let mut peer_strs = vec![];
        loop {
            let next = self.multisig_create_next(
                Some(peer_strs.clone()), Some(threshold), &wallet_filename).await?;
            if next == MoneroWalletMultisigRpcState::MultisigReadyToSend {
                break;
            }
            let info_str = next.multisig_info_string().ok_msg("No multisig info string from self")?;
            let mut req = Request::default();
            req.monero_multisig_formation_request = Some(MoneroMultisigFormationRequest {
                public_keys: all_pks.clone(),
                threshold: Some(Weighting::from_int_basis(threshold, all_pks.len() as i64)),
                peer_strings: peer_strs.clone(),
            });

            let mut new_peer_strs = vec![info_str];
            // TODO: fix this with retries and or elimination of a particular peer that is failing.
            for r in peer_broadcast.broadcast(&all_pks.clone(), req).await? {
                let r = r?;
                let r = r.monero_multisig_formation_response.ok_msg("No response from peer")?;
                new_peer_strs.push(r);
            }
            peer_strs = new_peer_strs;
        }
        self.get_secret()
    }

    pub fn get_wallet_filename_id(all_pks: &Vec<PublicKey>, threshold: i64) -> String {
        let mut wallet_ident = vec![];
        all_pks.iter().for_each(|pk| wallet_ident.extend(pk.vec()));
        threshold.to_le_bytes().iter().for_each(|b| wallet_ident.push(*b));
        let b = Hash::digest(wallet_ident).raw_bytes_hex().unwrap();
        b
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

    fn _id_to_filename(id: &MultipartyIdentifier) -> String {
        let string = id.proto_serialize_hex().last_n(12).unwrap();
        format!("{}_{}", Self::multisig_filename_prefix().unwrap(), string)
    }

    pub async fn multisig_send_prepare_and_sign(
        &mut self,
        amounts: Vec<(Address, CurrencyAmount)>
    ) -> RgResult<(TransferSplitResult, SignedMultisigTxset)> {
        let mut m: super::rpc_multisig::MoneroWalletRpcMultisigClient = self.wallet_rpc.get_multisig()?;

        let mut vec = vec![];

        for (addr, amount) in amounts.iter() {
            vec.push((addr.render_string()?, amount.amount as u64));
        }
        // Create the transaction
        let tx = m.transfer_split(vec, None, None).await?;
        
        // Sign our portion of the multisig transaction
        let signed = m.sign_multisig(tx.multisig_txset.clone()).await?;
        
        Ok((tx, signed))
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

    // Export multisig info for other participants
    pub async fn export_multisig_info(&mut self) -> RgResult<String> {
        let mut m = self.wallet_rpc.get_multisig()?;
        Ok(m.export_multisig_info().await?)
    }

    // Import multisig info from other participants
    pub async fn import_multisig_info(&mut self, info: Vec<String>) -> RgResult<u64> {
        let mut m = self.wallet_rpc.get_multisig()?;
        Ok(m.import_multisig_info(info).await?)
    }

    // Submit a fully signed multisig transaction
    pub async fn submit_multisig_transaction(&mut self, tx_data_hex: String) -> RgResult<Vec<String>> {
        let mut m = self.wallet_rpc.get_multisig()?;
        Ok(m.submit_multisig(tx_data_hex).await?)
    }

    // Helper function to describe a transaction before signing
    pub async fn describe_multisig_transaction(&mut self, unsigned_txset: String) -> RgResult<DescribeTransferResult> {
        let mut m = self.wallet_rpc.get_multisig()?;
        Ok(m.describe_transfer(unsigned_txset).await?)
    }

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

// #[ignore]
#[tokio::test]
async fn local_three_node() {

    if std::env::var("REDGOLD_DEBUG_DEVELOPER").is_err() {
        return;
    }

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
    
    let fnm = mpi.proto_serialize_hex().first_n(12).unwrap();
    
    let mut loop_vec = vec![];
    loop {
        let mut new_peer_strs = vec![];
        let mut last_ret = MoneroWalletMultisigRpcState::Unknown;
        let mut i = 0;
        for rpc in rpc_vecs.iter_mut() {
            let ret = rpc.multisig_create_next(
                Some(peer_strs.clone()),
                Some(2),
                &fnm
            ).await.unwrap();
    
            if i == 0 {
                loop_vec.push(ret.clone());
            }
            i += 1;
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
    
    let history = rpc_vecs[0].history.clone();
    //
    // let mut one_rpc_replicated = MoneroNodeRpcInterfaceWrapper::from_config(
    //     &one, s.clone(), "/disk/monerotw2","~/wallet.exp".to_string(), Some(true)
    // ).unwrap().unwrap();
    //
    // one_rpc_replicated.restore_from_history(history, &"test_filename_differenet".to_string()).await.unwrap();
    //
    // let addr = one_rpc_replicated.any_multisig_addr_creation().unwrap();
    // println!("Restored wallet address {}", addr);


    let msig = rpc_vecs[0].clone();
    let addr = msig.any_multisig_addr_creation().unwrap();
    let balance_of_multisig = msig.wallet_rpc.get_balance().await.unwrap();
    println!("Balance of multisig: {:?}", balance_of_multisig.to_fractional());
    
    println!("Done");
    
    four_rpc.wallet_rpc.register_self_activate_ok(Some("hot".to_string())).await.unwrap();
    // four_rpc.wallet_rpc.sync_info()
    let sync_info = four_rpc.wallet_rpc.refresh_sync_check_wallet().await.expect("refresh");
    println!("sync info done: {:?}", sync_info);
    // let refresh = rpc.client.clone().wallet().refresh(None).await.expect("refresh");
    let b = four_rpc.wallet_rpc.get_balance().await.unwrap();
    println!("Balance: {:?}", b);
    println!("Balance: {:?}", b.to_fractional());
    println!("Address {}", four_rpc.wallet_rpc.self_address_str().unwrap());

    let destinations = vec![
        (Address::from_monero_external(&addr),
        CurrencyAmount::from_fractional_cur(0.002f64, SupportedCurrency::Monero).unwrap())
    ];
    let tx = four_rpc.wallet_rpc.send(destinations).await.unwrap();
    println!("Tx: {}", tx);

    // let amt = b.to_fractional() / 10;

    // let (tx, signed) = four_rpc.multisig_send_prepare_and_sign(vec![(Address::from_monero_external(&addr), 
    // CurrencyAmount::from_fractional_cur(100_000_000_000, SupportedCurrency::Monero))]).await.unwrap();
    // println!("Tx: {}", tx);

}