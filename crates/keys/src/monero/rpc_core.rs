use crate::monero::key_derive::MoneroSeedBytes;
use crate::monero::rpc_multisig::MoneroWalletRpcMultisigClient;
use crate::word_pass_support::WordsPassNodeConfig;
use monero::util::address::Address;
use monero_rpc::monero::PrivateKey;
use monero_rpc::{GenerateFromKeysArgs, GetBlockHeaderSelector, GetTransfersCategory, GetTransfersSelector, RpcAuthentication, RpcClient, RpcClientBuilder, TransferHeight, WalletCreation};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::keys::words_pass::WordsPass;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::{CurrencyAmount, ErrorInfo, NetworkEnvironment, SupportedCurrency};
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use redgold_schema::{ErrorInfoContext, RgResult, SafeOption, ShortString};
use std::num::NonZeroU64;
use std::str::FromStr;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct MoneroRpcWrapper {
    pub url: String,
    pub network: NetworkEnvironment,
    pub client: RpcClient,
    pub auth_str: Option<String>,
    pub nc: NodeConfig
}

impl MoneroRpcWrapper {

    pub fn get_multisig(&self) -> RgResult<MoneroWalletRpcMultisigClient> {
        MoneroWalletRpcMultisigClient::new(self.url.clone(), self.auth_str.clone())
    }

    pub fn from_config(cfg: &NodeConfig) -> Option<RgResult<Self>> {
        let url = cfg.rpc_url(SupportedCurrency::Monero).into_iter()
            .filter(|r| !r.wallet_only.unwrap_or(false))
            .next();
        if let Some(rpc) = url {
            let network = cfg.network.clone();
            let ret = MoneroRpcWrapper::new(rpc.url.clone(), network, None, cfg);
            Some(ret)
        } else {
            None
        }
    }

    pub fn self_address(&self) -> RgResult<redgold_schema::structs::Address> {
        self.nc.words().monero_external_address(&self.nc.network)
    }

    pub fn self_address_str(&self) -> RgResult<String> {
        self.self_address().and_then(|a| a.render_string())
    }

    pub fn view_key(&self) -> RgResult<String> {
        self.nc.words().derive_monero_keys().map(|kp| kp.view.to_string())
    }

    pub fn spend_key(&self) -> RgResult<String> {
        self.nc.words().derive_monero_keys().map(|kp| kp.spend.to_string())
    }


    pub fn authed_from_config(cfg: &NodeConfig) -> Option<RgResult<Self>> {
        let url = cfg.rpc_url(SupportedCurrency::Monero).into_iter()
            .filter(|r| r.wallet_only.unwrap_or(false))
            .next();
        if let Some(rpc) = url {
            let network = cfg.network.clone();
            // println!("rpc authed: {:?}", rpc);
            let ret = MoneroRpcWrapper::new(rpc.url.clone(), network, rpc.authentication.clone(), cfg);
            Some(ret)
        } else {
            None
        }
    }

    pub fn new(url: String, network: NetworkEnvironment, auth_str: Option<String>, nc: &NodeConfig) -> RgResult<Self> {
        let builder = RpcClientBuilder::new();
        let authed = if let Some(a) = auth_str.as_ref() {
            let user = a.split(":").next().unwrap_or("").to_string();
            let pass = a.split(":").last().unwrap_or("").to_string();
            let auth = RpcAuthentication::Credentials { username: user, password: pass };
            // println!("authed: {:?}", auth.clone());
            builder.rpc_authentication(auth.clone())
        } else {
            builder
        };
        let client = authed
            .build(url.clone())
            .map_err(|e| ErrorInfo::new(format!("Failed to create Monero RPC client {}", e.to_string())))?;
        let ret = MoneroRpcWrapper {
            url,
            network,
            client,
            auth_str,
            nc: nc.clone(),
        };
        // println!("new rpc: {:?}", ret);
        Ok(ret)
    }


    pub fn check_error_message_already_registered(res: &RgResult<WalletCreation>) -> bool {
        match res {
            Ok(_) => {}
            Err(e) => {
                return e.message.contains("Wallet already exists");
            }
        }
        false
    }


    pub async fn register_dupe_ok(
        &self,
        view_key: String,
        address: String,
        spend_key: Option<String>,
        password: Option<String>,
        wallet_pfx: Option<String>,
        wallet_filename: Option<String>,
    ) -> RgResult<()> {
        // println!("Registering key: {}", view_key);
        // println!("Registering key address: {}", address);
        let res = self.register_key(view_key, address, spend_key, password, wallet_pfx, wallet_filename).await;
        if MoneroRpcWrapper::check_error_message_already_registered(&res) {
            Ok(())
        } else {
            res.map(|_| ())
        }
    }

    pub async fn register_self_activate_ok(&self, wallet_filename: Option<String>) -> RgResult<()> {
        self.register_dupe_ok(self.view_key()?, self.self_address_str()?, Some(self.spend_key()?), None, None, wallet_filename.clone()).await?;
        self.open_wallet_filename(wallet_filename.clone().ok_msg("Missing wallet filename")?).await
    }

    pub async fn register_key(
        &self,
        view_key: String,
        address: String,
        spend_key: Option<String>,
        password: Option<String>,
        wallet_pfx: Option<String>,
        wallet_filename: Option<String>,
    ) -> RgResult<WalletCreation> {
        let password = password.unwrap_or("".to_string());
        let filename = address.last_n(12)?;
        let filename = wallet_pfx.map(|p| format!("{}{}", p, filename)).unwrap_or(filename);
        let filename = wallet_filename.unwrap_or(filename);
        let address = Address::from_str(&address)
            .error_info("Invalid Monero address")?;
        let pk = PrivateKey::from_str(&view_key)
            .error_info("Invalid Monero view key")?;
        let client = self.client.clone().wallet();

        let spend_key = match spend_key {
            Some(s) => Some(PrivateKey::from_str(&s)
                .error_info("Invalid Monero spend key")?),
            None => None,
        };

        let args = GenerateFromKeysArgs {
            restore_height: None,
            filename,
            address: address,
            spendkey: spend_key,
            viewkey: pk,
            password,
            autosave_current: None,
        };
        // println!("Registering on url: {} auth: {}", self.url, self.auth_str.clone().unwrap_or("".to_string()));
        // println!("Registering with args: {:?}", args);
        let response = client.generate_from_keys(
            args
        ).await
            // .unwrap();
            .map_err(|e| ErrorInfo::new(format!("Failed to register key {}", e.to_string())))?;
        // println!("response: {:?}", response);
        // .error_info("Failed to register Monero key")?;
        Ok(response)
    }

    pub async fn get_all_transactions(&self) -> RgResult<Vec<ExternalTimedTransaction>> {
        let mut hm = std::collections::HashMap::new();
        hm.insert(GetTransfersCategory::In, true);
        hm.insert(GetTransfersCategory::Out, true);
        // hm.insert(GetTransfersCategory::Pending, true);
        // hm.insert(GetTransfersCategory::Failed, true);
        let res = self.client.clone().wallet().get_transfers(
            GetTransfersSelector {
                category_selector: hm,
                account_index: None,
                subaddr_indices: None,
                block_height_filter: None,
            }
        ).await
            .map_err(|e| ErrorInfo::new(format!("Failed to get all {}", e.to_string())))?;
        let mut results = vec![];
        for (k,v) in res.iter() {
            for vv in v.iter() {
                let mut ett = ExternalTimedTransaction::default();
                ett.tx_id = vv.txid.to_string();
                ett.timestamp = Some(vv.timestamp.timestamp_millis() as u64);
                ett.other_address = vv.address.to_string();
                ett.amount = vv.amount.as_pico();
                ett.currency = SupportedCurrency::Monero;
                ett.block_number = match vv.height {
                    TransferHeight::Confirmed(h) => Some(h.get()),
                    TransferHeight::InPool => None,
                };
                ett.incoming = k == &GetTransfersCategory::In;
                ett.fee = Some(CurrencyAmount::from_currency(vv.fee.as_pico() as i64, SupportedCurrency::Monero));
                results.push(ett);
            }
        }
        Ok(results)
    }

    pub async fn open_wallet_filename(&self, filename: String) -> RgResult<()> {
        let res = self.client.clone().wallet()
            .open_wallet(filename.clone(), None).await
            .map_err(|e| ErrorInfo::new(format!("Failed to activate wallet {}", e.to_string())))
            .with_detail("wallet", filename.clone());
        res
    }

    pub async fn activate_wallet(&self, address: String, prefix: Option<String>) -> RgResult<()> {
        let filename = address.last_n(12)?;
        let filename = prefix.map(|p| format!("{}{}", p, filename)).unwrap_or(filename);
        let res = self.open_wallet_filename(filename).await
        .with_detail("address", address.clone());
        res
    }

    pub async fn sync_info(&self) -> RgResult<NonZeroU64> {
        let sync_info = self.client.clone().wallet().get_height().await
            .map_err(|e| ErrorInfo::new(format!("Failed to get height {}", e.to_string())))?;
        Ok(sync_info)
    }

    pub async fn refresh_sync_check_wallet(&self) -> RgResult<NonZeroU64> {
        loop {
            let sync_info = self.sync_info().await?;
            tokio::time::sleep(Duration::from_secs(5)).await;
            let sync_info_2 = self.sync_info().await?;
            if sync_info == sync_info_2 {
                return Ok(sync_info)
            }
            println!("Waiting for wallet to sync...from {} to {}", sync_info, sync_info_2);
            tokio::time::sleep(Duration::from_secs(10)).await;
        };
    }

    pub async fn close_wallet(&self) -> RgResult<()> {
        let res = self.client.clone().wallet().close_wallet().await
            .map_err(|e| ErrorInfo::new(format!("Failed to close wallet {}", e.to_string())));
        res
    }

    pub async fn get_balance(&self) -> RgResult<CurrencyAmount> {
        // self.client.clone().wallet()
        let b = self.client.clone().wallet().get_balance(0, None)
            .await
            .map_err(|e| ErrorInfo::new(format!("Failed to get balance {}", e.to_string())))?;
        println!("balance: {:?}", b);
        Ok(CurrencyAmount::from_currency(b.balance.as_pico() as i64, SupportedCurrency::Monero))
    }



}


#[ignore]
#[tokio::test]
async fn check_rpc_wallet() {

    // let ci = dev_ci_kp().unwrap();
    let w = std::env::var("REDGOLD_TEST_WORDS").unwrap();
    let w = WordsPass::new(w, None);
    // This is wrong for ethereum, but we just need the secret key to match other
    // faucet funds for 'same' key.
    // let path = "m/84'/0'/0'/0/0";
    let a = w.derive_monero_address(&NetworkEnvironment::Dev).expect("address");
    println!("address: {}", a.to_string());
    // address: 9wi5fzpi5uVENtzG1A6fpVQooVkxoXJokJuz1MZzMzVK4XfhULjDEVB8UGpfHhFpgXBkBbUeRdKEZJArLJqR3ZF3UJutgws
    // Amount sent: 1 XMR
    // Transaction ID: f501643c8c2d16a7f1abff1cee4cc7b894edcc274b4117bf0b7ca98f3e4fc451
    let kp = w.derive_monero_keys().expect("keys");
    println!("view key: {}", kp.view.to_string());
    let rpc = MoneroRpcWrapper::new(
        "http://server:28088".to_string(),
        NetworkEnvironment::Dev,
        Some("username:password".to_string()),
        &NodeConfig::default()
    ).expect("rpc"); // Some(kp.spend.to_string())
    let pfx = Some("randomtest".to_string());
    // let tx = rpc.register_dupe_ok(kp.view.to_string(), a.to_string(), None, None, pfx.clone()).await.expect("works");
    // tokio::time::sleep(Duration::from_secs(1)).await;
    rpc.activate_wallet(a.to_string(), pfx.clone()).await.expect("activate");
    let sync_info = rpc.client.clone().wallet().get_height().await.expect("height check");
    println!("sync info direct: {:?}", sync_info);
    let sync_info = rpc.refresh_sync_check_wallet().await.expect("refresh");
    println!("sync info done: {:?}", sync_info);
    let refresh = rpc.client.clone().wallet().refresh(None).await.expect("refresh");
    // println!("refresh: {:?}", refresh);
    //
    //
    // // Wait for wallet sync and check status
    // let mut synced = false;
    // let mut attempts = 0;
    // while !synced && attempts < 10 {
    //     let daemon_height = rpc.client.clone().daemon().get_block_count().await.expect("daemon height");
    //
    //     println!("Wallet height: {}, Daemon height: {}", sync_info, daemon_height);
    //
    //     if sync_info >= daemon_height {
    //         synced = true;
    //     } else {
    //         println!("Waiting for wallet to sync...");
    //         tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    //         attempts += 1;
    //     }
    // }
    //
    // if !synced {
    //     panic!("Wallet failed to sync after multiple attempts");
    // }


    // println!("tx: {:?}", tx.clone());
    // let already = MoneroRpcWrapper::check_error_message_already_registered(tx);
    let b=  rpc.get_balance().await.expect("balance");
    println!("balance: {:?}", b);
}

#[ignore]
#[tokio::test]
async fn check_rpc_manually() {
    let rpc = MoneroRpcWrapper::new("http://server:28089".to_string(), NetworkEnvironment::Dev, None, &NodeConfig::default()).expect("rpc");
    let h = monero::Hash::from_str("f501643c8c2d16a7f1abff1cee4cc7b894edcc274b4117bf0b7ca98f3e4fc451")
    // let h = monero::Hash::from_str("eb266f3acb2e66c7510b3e2ee48d50ba3c2deba1a8647c36ff1bc72b72b2cbce")
        .unwrap();
    let tx = rpc.client.clone().daemon_rpc().get_transactions(
        vec![h], Some(true), None
    ).await.expect("works");
    println!("tx: {:?}", tx);
    let h = rpc.client.clone().daemon().get_block_count().await.expect("height check");
    println!("height: {:?}", h);
    let hh = rpc.client.clone().daemon().get_block_header(GetBlockHeaderSelector::Last).await.expect("header");
    println!("header: {:?}", hh);
}