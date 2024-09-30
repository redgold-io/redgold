use redgold_schema::config_data::ConfigData;
use redgold_schema::RgResult;
use redgold_schema::structs::{AboutNodeResponse, AddressInfo, PublicKey, SubmitTransactionResponse, Transaction};
use redgold_schema::tx::tx_builder::TransactionBuilder;
use redgold_schema::util::times::current_time_millis;
use crate::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};

pub struct WasmGuiDepends {

}

impl GuiDepends for WasmGuiDepends {
    fn get_salt(&self) -> i64 {
        let random = current_time_millis();
        random
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
        todo!()
    }

    fn sign_transaction(&self, tx: &Transaction, sign_info: &TransactionSignInfo) -> RgResult<Transaction> {
        todo!()
    }
}