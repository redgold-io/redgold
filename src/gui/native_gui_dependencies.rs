use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use flume::{Receiver, Sender};
use futures::future::join_all;
use itertools::all;
use rand::Rng;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_common_no_wasm::tx_new::TransactionBuilderSupport;
use redgold_gui::components::balance_table::queryable_balances;
use redgold_gui::components::tx_progress::PreparedTransaction;
use redgold_gui::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};
use redgold_gui::state::local_state::{LocalStateUpdate, PricesPartyInfoAndDeltaInitialQuery};
use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};
use redgold_keys::address_support::AddressSupport;
use redgold_keys::KeyPair;
use redgold_keys::proof_support::PublicKeySupport;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_keys::xpub_wrapper::{ValidateDerivationPath, XpubWrapper};
use redgold_ops::backup_datastore::{backup_datastore_servers, restore_datastore_servers};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::config_data::ConfigData;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::explorer::DetailedAddress;
use redgold_schema::conf::local_stored_state::AccountKeySource;
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::{ErrorInfoContext, RgResult, SafeOption};
use redgold_schema::observability::errors::Loggable;
use redgold_schema::structs::{AboutNodeResponse, Address, AddressInfo, ErrorInfo, NetworkEnvironment, PublicKey, SubmitTransactionResponse, SupportedCurrency, Transaction};
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use redgold_schema::tx::tx_builder::TransactionBuilder;
use crate::core::relay::Relay;
use crate::gui::components::tx_signer::{TxBroadcastProgress, TxSignerProgress};
use crate::integrations::external_network_resources::ExternalNetworkResourcesImpl;
use crate::node_config::ApiNodeConfig;
use crate::scrape::get_24hr_delta_change_pct;
use crate::util;

#[derive(Clone)]
pub struct NativeGuiDepends {
    pub original_uncleared_nc: NodeConfig,
    pub nc: Arc<Mutex<NodeConfig>>,
    wallet_nw: HashMap<NetworkEnvironment, ExternalNetworkResourcesImpl>,
    network_changed_sender: Sender<NetworkEnvironment>,
    network_changed: Receiver<NetworkEnvironment>,
}

impl NativeGuiDepends {
    pub fn new(nc: NodeConfig) -> Self {
        let (network_changed_sender, network_changed) = flume::unbounded();
        Self {
            original_uncleared_nc: nc.clone(),
            nc: Arc::new(Mutex::new(nc)),
            wallet_nw: Default::default(),
            network_changed_sender,
            network_changed,
        }
    }

    fn external_res(&mut self) -> Result<ExternalNetworkResourcesImpl, ErrorInfo> {
        let eee = if let Some(e) = self.wallet_nw.get(&self.nc().network) {
            e
        } else {
            let e = ExternalNetworkResourcesImpl::new(&self.original_uncleared_nc, None)?;
            self.wallet_nw.insert(self.nc().network.clone(), e);
            self.wallet_nw.get(&self.nc().network).unwrap()
        };
        Ok(eee.clone())
    }
    fn nc(&self) -> NodeConfig {
        self.nc.lock().unwrap().clone()
    }
}

impl GuiDepends for NativeGuiDepends {

    fn network_changed(&self) -> flume::Receiver<NetworkEnvironment> {
        self.network_changed.clone()
    }


    fn config_df_path_label(&self) -> Option<String> {
        self.nc().secure_or().path.to_str().map(|s| s.to_string())
    }

    fn get_salt(&self) -> i64 {
        let mut rng = rand::thread_rng();
        rng.gen::<i64>()
    }

    fn get_config(&self) -> ConfigData {
        (*self.nc().config_data).clone()
    }

    fn set_config(&mut self, config: &ConfigData, allow_overwrite_all: bool) {
        let mut config = config.clone();
        if !allow_overwrite_all {
            config.network = None;
            config.home = None;
            config.config = None;
        }
        let l = config.local.get_or_insert(Default::default());
        let k = l.keys.get_or_insert(Default::default());
        k.retain(|k| k.skip_persist.map(|x| !x).unwrap_or(true));
        l.mnemonics.as_mut().map(|m| {
            m.retain(|m| m.persist_disk.map(|x| x).unwrap_or(true));
        });

        if !allow_overwrite_all {
            config.node.get_or_insert(Default::default()).words = None;
            let sec = config.secure.get_or_insert(Default::default());
            sec.path = None;
        }
        let mut nc = self.nc();
        nc.config_data = Arc::new(config.clone());
        self.nc = Arc::new(Mutex::new(nc));
        self.nc().secure_or().write_config(&config).unwrap();
    }

    async fn get_address_info(&self, pk: &PublicKey) -> RgResult<AddressInfo> {
        self.nc().api_rg_client().address_info_for_pk(pk).await
    }

    async fn submit_transaction(&self, tx: &Transaction) -> RgResult<SubmitTransactionResponse> {
        self.nc().api_client().send_transaction(tx, true).await
    }

    async fn metrics(&self) -> RgResult<Vec<(String, String)>> {
        self.nc().api_rg_client().metrics().await
    }

    async fn table_sizes(&self) -> RgResult<Vec<(String, i64)>> {
        self.nc().api_rg_client().table_sizes().await
    }

    async fn about_node(&self) -> RgResult<AboutNodeResponse> {
        self.nc().api_rg_client().about().await
    }

    fn tx_builder(&self) -> TransactionBuilder {
        TransactionBuilder::new(&self.nc())
    }

    fn sign_prepared_transaction(&mut self, tx: &PreparedTransaction, results: flume::Sender<RgResult<PreparedTransaction>>) -> RgResult<()> {
        let ext = self.external_res()?.clone();
        let p = tx.clone();
        let self_clone = self.clone();
        self.spawn(async move {
            let res = p.sign(ext, self_clone).await;
            results.send(res).unwrap();
        });
        Ok(())
    }

    fn broadcast_prepared_transaction(&mut self, tx: &PreparedTransaction, results: flume::Sender<RgResult<PreparedTransaction>>) -> RgResult<()> {
        let ext = self.external_res()?.clone();
        let p = tx.clone();
        self.spawn(async move {
            let res = p.broadcast(ext).await;
            results.send(res).unwrap();
        });
        Ok(())
    }

    fn sign_transaction(&self, tx: &Transaction, sign_info: &TransactionSignInfo) -> RgResult<Transaction> {
        match sign_info {
            TransactionSignInfo::PrivateKey(str) => {
                let mut tx = tx.clone();
                let mut signed = tx.sign(&KeyPair::from_private_hex(str.clone())?)?;
                Ok(signed.with_hashes().clone())
            }
            TransactionSignInfo::Mnemonic(m) => {
                let str = WordsPass::new(m.words.clone(), m.passphrase.clone())
                    .private_at(m.clone().path.ok_msg("Path not provided")?)?;
                let mut tx = tx.clone();
                let mut signed = tx.sign(&KeyPair::from_private_hex(str.clone())?)?;
                Ok(signed.with_hashes().clone())
            }
            _ => "Unimplemented".to_error()
            // TransactionSignInfo::ColdHardwareWallet(_) => {}
        }
    }

    fn spawn(&self, f: impl Future<Output=()> + Send + 'static) {
        tokio::spawn(f);
    }

    fn validate_derivation_path(&self, derivation_path: impl Into<String>) -> bool {
        derivation_path.into().valid_derivation_path()
    }

    async fn s3_checksum(&self) -> RgResult<String> {
        let s3_release_exe_hash = util::auto_update::
        get_s3_sha256_release_hash_short_id(self.nc().network.clone(), None).await;
        s3_release_exe_hash
    }

    fn set_network(&mut self, network: &NetworkEnvironment) {
        let mut nc = (self.nc()).clone();
        if nc.network != network.clone() {
            self.network_changed_sender.send(network.clone()).unwrap();
        }
        nc.network = network.clone();
        self.nc = Arc::new(Mutex::new(nc));
    }

    async fn get_address_info_multi(&self, pk: Vec<&PublicKey>) -> Vec<RgResult<AddressInfo>> {
        let client = Arc::new(self.nc().api_rg_client());

        let futures = pk.iter().map(|pk| {
            let client = Arc::clone(&client);
            async move {
                client.address_info_for_pk(pk).await
            }
        });

        join_all(futures).await
    }

    async fn party_data(&self) -> RgResult<HashMap<PublicKey, PartyInternalData>> {
        self.nc().api_rg_client().party_data().await
    }

    fn xpub_public(&self, xpub: String, path: String) -> RgResult<PublicKey> {
        XpubWrapper::new(xpub).public_at_dp(&path)
    }

    async fn get_24hr_delta(&self, currency: SupportedCurrency) -> f64 {
        get_24hr_delta_change_pct(currency).await.unwrap_or(0.0)
    }

    async fn get_detailed_address(&self, pk: &PublicKey) -> RgResult<Vec<DetailedAddress>> {
        self.nc().api_rg_client().explorer_public_address(pk).await
    }

    async fn get_external_tx(&mut self, pk: &PublicKey, currency: SupportedCurrency) -> RgResult<Vec<ExternalTimedTransaction>> {
        let eee = self.external_res()?;
        eee.get_all_tx_for_pk(pk, currency, None).await
    }

    fn get_network(&self) -> NetworkEnvironment {
        self.nc().network
    }

    fn parse_address(&self, address: impl Into<String>) -> RgResult<Address> {
        address.into().parse_address()
    }

    fn to_all_address(&self, pk: &PublicKey) -> Vec<Address> {
        pk.to_all_addresses_for_network(&self.nc().network).unwrap_or_default()
    }

    fn spawn_blocking<T: Send + 'static>(&self, f: impl Future<Output=RgResult<T>> + Send + 'static) -> RgResult<T> {
        tokio::runtime::Handle::current().block_on(f)
    }

    fn form_eth_address(&self, pk: &PublicKey) -> RgResult<Address> {
        pk.to_ethereum_address_typed()
    }

    fn spawn_interrupt(&self, f: impl Future<Output=()> + Send + 'static, interrupt: Receiver<()>) {
        tokio::spawn(async {
            let mut result = tokio::spawn(f);
            tokio::select! {
            _ = &mut result => {},
            _ = interrupt.into_recv_async() => {
                    result.abort();
                },
            }
        });
    }

    fn initial_queries_prices_parties_etc<E>(&self, sender: Sender<LocalStateUpdate>, ext: E) -> ()
    where E: ExternalNetworkResources + Send + 'static + Clone {
        if self.nc().offline() {
            return;
        }
        let g2 = self.clone();
        // let e2 = ext.clone();

        let client = self.nc().api_rg_client();
        self.spawn(async move {
            let result = client.party_data().await;

            let mut price_map: HashMap<SupportedCurrency, f64> = Default::default();
            for c in queryable_balances() {
                if c == SupportedCurrency::Redgold {
                    continue;
                }
                if let Some(p) = ext.query_price(util::current_time_millis_i64(), c).await.log_error().ok() {
                    price_map.insert(c, p);
                }
            }

            let cpp = result.as_ref().ok()
                .and_then(|x| x.iter().next())
                .map(|x| x.1)
                .and_then(|p| p.party_events.as_ref())
                .and_then(|pe| pe.central_prices.get(&SupportedCurrency::Ethereum))
                .map(|c| c.min_bid_estimated.clone())
                .unwrap_or(100.0);
            price_map.insert(SupportedCurrency::Redgold, cpp);

            let party = result.unwrap_or_default();


            let mut deltas = HashMap::default();
            for cur in vec![
                SupportedCurrency::Ethereum, SupportedCurrency::Bitcoin, SupportedCurrency::Usdt, SupportedCurrency::Solana, SupportedCurrency::Monero, SupportedCurrency::Usdc
            ].iter() {
                let delta = g2.get_24hr_delta(cur.clone()).await;
                deltas.insert(cur.clone(), delta);
            }
            sender.send(LocalStateUpdate::PricesPartyInfoAndDelta(PricesPartyInfoAndDeltaInitialQuery {
                prices: price_map,
                party_info: party,
                delta_24hr: deltas,
                daily_one_year: ext.daily_historical_year().await.ok().unwrap_or_default(),
            })).ok();
        });
    }
    
    
    fn form_btc_address(&self, pk: &PublicKey) -> RgResult<Address> {
        pk.to_bitcoin_address_typed(&self.get_network())
    }

    fn backup_data_stores(&self) -> RgResult<()> {
        let nc = self.nc.lock().unwrap().clone();
        let servers = nc.servers_old();
        self.spawn(async {
            backup_datastore_servers(nc, servers).await
        });
        Ok(())
    }

    fn restore_data_stores(&self) -> RgResult<()> {
        let nc = self.nc.lock().unwrap().clone();
        let servers = nc.servers_old();
        self.spawn(async {
            restore_datastore_servers(nc, servers).await
        });
        Ok(())
    }

}
