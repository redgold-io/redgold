use rand::Rng;
use redgold_common_no_wasm::tx_new::TransactionBuilderSupport;
use redgold_gui::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};
use redgold_keys::KeyPair;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::config_data::ConfigData;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::RgResult;
use redgold_schema::structs::{AboutNodeResponse, AddressInfo, PublicKey, SubmitTransactionResponse, Transaction};
use redgold_schema::tx::tx_builder::TransactionBuilder;
use crate::core::relay::Relay;

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
    fn get_salt(&self) -> i64 {
        let mut rng = rand::thread_rng();
        rng.gen::<i64>()
    }

    async fn get_config(&self) -> ConfigData {
        todo!()
    }

    async fn set_config(&self, config: ConfigData) {
        todo!()
    }

    async fn get_address_info(&self, pk: &PublicKey) -> RgResult<Option<AddressInfo>> {
        todo!()
    }

    async fn submit_transaction(&self, tx: &Transaction) -> RgResult<SubmitTransactionResponse> {
        todo!()
    }

    async fn about_node(&self) -> RgResult<AboutNodeResponse> {
        todo!()
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
}
