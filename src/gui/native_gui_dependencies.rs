use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use futures::future::join_all;
use rand::Rng;
use redgold_common_no_wasm::tx_new::TransactionBuilderSupport;
use redgold_gui::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};
use redgold_keys::KeyPair;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::xpub_wrapper::{ValidateDerivationPath, XpubWrapper};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::config_data::ConfigData;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::local_stored_state::NamedXpub;
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::RgResult;
use redgold_schema::structs::{AboutNodeResponse, AddressInfo, NetworkEnvironment, PublicKey, SubmitTransactionResponse, Transaction};
use redgold_schema::tx::tx_builder::TransactionBuilder;
use crate::core::relay::Relay;
use crate::node_config::ApiNodeConfig;
use crate::util;

#[derive(Clone)]
pub struct NativeGuiDepends {
    nc: NodeConfig
}

impl NativeGuiDepends {
    pub fn new(nc: NodeConfig) -> Self {
        Self {
            nc
        }
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

    fn sign_transaction(&self, tx: &Transaction, sign_info: &TransactionSignInfo) -> RgResult<Transaction> {
        match sign_info {
            TransactionSignInfo::PrivateKey(str) => {
                let mut tx = tx.clone();
                let signed = tx.sign(&KeyPair::from_private_hex(str.clone())?);
                signed
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
}
