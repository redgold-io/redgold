use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use flume::Sender;
use futures::future::join_all;
use rand::Rng;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_common_no_wasm::tx_new::TransactionBuilderSupport;
use redgold_gui::components::tx_progress::PreparedTransaction;
use redgold_gui::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};
use redgold_keys::address_external::ToEthereumAddress;
use redgold_keys::address_support::AddressSupport;
use redgold_keys::KeyPair;
use redgold_keys::proof_support::PublicKeySupport;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::xpub_wrapper::{ValidateDerivationPath, XpubWrapper};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::config_data::ConfigData;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::explorer::DetailedAddress;
use redgold_schema::conf::local_stored_state::NamedXpub;
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::{ErrorInfoContext, RgResult};
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
    nc: NodeConfig,
    wallet_nw: HashMap<NetworkEnvironment, ExternalNetworkResourcesImpl>
}

impl NativeGuiDepends {
    pub fn new(nc: NodeConfig) -> Self {
        Self {
            nc,
            wallet_nw: Default::default(),
        }
    }

    fn external_res(&mut self) -> Result<ExternalNetworkResourcesImpl, ErrorInfo> {
        let eee = if let Some(e) = self.wallet_nw.get(&self.nc.network) {
            e
        } else {
            let e = ExternalNetworkResourcesImpl::new(&self.nc, None)?;
            self.wallet_nw.insert(self.nc.network.clone(), e);
            self.wallet_nw.get(&self.nc.network).unwrap()
        };
        Ok(eee.clone())
    }
}

impl GuiDepends for NativeGuiDepends {
    fn config_df_path_label(&self) -> Option<String> {
        self.nc.secure_or().path.to_str().map(|s| s.to_string())
    }

    fn get_salt(&self) -> i64 {
        let mut rng = rand::thread_rng();
        rng.gen::<i64>()
    }

    fn get_config(&self) -> ConfigData {
        self.nc.secure_or().config().unwrap().unwrap()
    }

    fn set_config(&self, config: &ConfigData) {
        self.nc.secure_or().write_config(config).unwrap();
    }

    async fn get_address_info(&self, pk: &PublicKey) -> RgResult<AddressInfo> {
        self.nc.api_rg_client().address_info_for_pk(pk).await
    }

    async fn submit_transaction(&self, tx: &Transaction) -> RgResult<SubmitTransactionResponse> {
        self.nc.api_client().send_transaction(tx, true).await
    }

    async fn metrics(&self) -> RgResult<Vec<(String, String)>> {
        self.nc.api_rg_client().metrics().await
    }

    async fn table_sizes(&self) -> RgResult<Vec<(String, i64)>> {
        self.nc.api_rg_client().table_sizes().await
    }

    async fn about_node(&self) -> RgResult<AboutNodeResponse> {
        self.nc.api_rg_client().about().await
    }

    fn tx_builder(&self) -> TransactionBuilder {
        TransactionBuilder::new(&self.nc)
    }

    fn sign_prepared_transaction(&mut self, tx: &PreparedTransaction, results: flume::Sender<RgResult<PreparedTransaction>>) -> RgResult<()> {
        let mut ext = self.external_res()?.clone();
        let p = tx.clone();
        let self_clone = self.clone();
        self.spawn(async move {
            let res = p.sign(ext, self_clone).await;
            results.send(res).unwrap();
        });
        Ok(())
    }

    fn broadcast_prepared_transaction(&mut self, tx: &PreparedTransaction, results: flume::Sender<RgResult<PreparedTransaction>>) -> RgResult<()> {
        let mut ext = self.external_res()?.clone();
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
            _ => "Unimplemented".to_error()
            // TransactionSignInfo::ColdHardwareWallet(_) => {}
            // TransactionSignInfo::Qr(_) => {}
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
        get_s3_sha256_release_hash_short_id(self.nc.network.clone(), None).await;
        s3_release_exe_hash
    }

    fn set_network(&mut self, network: &NetworkEnvironment) {
        self.nc.network = network.clone();
    }

    async fn get_address_info_multi(&self, pk: Vec<&PublicKey>) -> Vec<RgResult<AddressInfo>> {
        let client = Arc::new(self.nc.api_rg_client());

        let futures = pk.iter().map(|pk| {
            let client = Arc::clone(&client);
            async move {
                client.address_info_for_pk(pk).await
            }
        });

        join_all(futures).await
    }

    async fn party_data(&self) -> RgResult<HashMap<PublicKey, PartyInternalData>> {
        self.nc.api_rg_client().party_data().await
    }

    fn xpub_public(&self, xpub: String, path: String) -> RgResult<PublicKey> {
        XpubWrapper::new(xpub).public_at_dp(&path)
    }

    async fn get_24hr_delta(&self, currency: SupportedCurrency) -> f64 {
        get_24hr_delta_change_pct(currency).await.unwrap_or(0.0)
    }

    async fn get_detailed_address(&self, pk: &PublicKey) -> RgResult<Vec<DetailedAddress>> {
        self.nc.api_rg_client().explorer_public_address(pk).await
    }

    async fn get_external_tx(&mut self, pk: &PublicKey, currency: SupportedCurrency) -> RgResult<Vec<ExternalTimedTransaction>> {
        let eee = self.external_res()?;
        eee.get_all_tx_for_pk(pk, currency, None).await
    }

    fn get_network(&self) -> &NetworkEnvironment {
        &self.nc.network
    }

    fn parse_address(&self, address: impl Into<String>) -> RgResult<Address> {
        address.into().parse_address()
    }

    fn to_all_address(&self, pk: &PublicKey) -> Vec<Address> {
        pk.to_all_addresses_for_network(&self.nc.network).unwrap_or_default()
    }

    fn spawn_blocking<T: Send + 'static>(&self, f: impl Future<Output=RgResult<T>> + Send + 'static) -> RgResult<T> {
        tokio::runtime::Handle::current().block_on(f)
    }

    fn form_eth_address(&self, pk: &PublicKey) -> RgResult<Address> {
        pk.to_ethereum_address_typed()
    }
}
