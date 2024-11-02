use std::path::PathBuf;
use async_trait::async_trait;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_gui::airgap::signer_window::AirgapTransport;
use redgold_gui::components::tx_progress::{PreparedTransaction, TransactionProgressFlow};
use redgold_gui::dependencies::gui_depends::{GuiDepends, TransactionSignInfo};
use redgold_keys::KeyPair;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_schema::{RgResult, SafeOption};
use redgold_schema::airgap::AirgapMessage;
use redgold_schema::conf::local_stored_state::XPubLikeRequestType;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::structs::{PublicKey, SupportedCurrency};
use crate::core::transact::tx_broadcast_support::TxBroadcastSupport;
use crate::util::current_time_unix;

#[async_trait]
pub trait TxSignerProgress {

    async fn sign<T, G>(
        &self,
        external_resources: T,
        g: G
    ) -> RgResult<PreparedTransaction>
    where T: ExternalNetworkResources + Send, G: GuiDepends + Send + Clone;
}


#[async_trait]
pub trait TxBroadcastProgress {

    async fn broadcast<T>(
        &self,
        external_resources: T,
    ) -> RgResult<PreparedTransaction> where T: ExternalNetworkResources + Send ;

}


#[async_trait]
impl TxSignerProgress for PreparedTransaction {

    async fn sign<T, G>(
        &self,
        external_resources: T,
        g: G
    ) -> RgResult<PreparedTransaction>
    where T: ExternalNetworkResources + Send, G: GuiDepends + Send + Clone {
        let mut updated = self.clone();
        match self.currency {
            SupportedCurrency::Redgold => {
                let mut tx = self.tx.safe_get_msg("Missing transaction")?.clone();

                match self.tsi.clone() {
                    TransactionSignInfo::PrivateKey(secret) => {
                        let kp = KeyPair::from_private_hex(secret.clone())?;
                        let transaction = tx.sign(&kp)?;
                        updated.signed_hash = transaction.signed_hash().hex();
                        updated.tx = Some(transaction);
                    }
                    TransactionSignInfo::ColdOrAirgap(h) => {
                        let msg = AirgapMessage::sign(h.path.clone(), tx.clone());
                        let mut transport = AirgapTransport::default();
                        let mut await_airgap = false;
                        match &self.signing_method {
                            XPubLikeRequestType::Cold => {
                                let tx = external_resources.trezor_sign(
                                    self.from.clone(), h.path, tx.clone()
                                ).await?;
                                updated.tx = Some(tx);
                            }
                            XPubLikeRequestType::Hot => { panic!("Hot signing not supported for Redgold within cold sign workflow") }
                            XPubLikeRequestType::QR => {
                                let stored_capture = g.get_config().secure.and_then(|s| s.capture_device_name);
                                transport = AirgapTransport::Qr(stored_capture);
                            }
                            XPubLikeRequestType::File => {
                                let file = updated.file_input.clone();
                                transport = AirgapTransport::File(file);
                            }
                        }
                        updated.airgap_signer_window.initialize_with(msg, transport);
                    }
                    TransactionSignInfo::Mnemonic(_) => {}
                }
            }
            _ => {
                match &self.tsi {
                    TransactionSignInfo::PrivateKey(_) => {
                        updated.signed_hash = updated.unsigned_hash.clone();
                    }
                    _ => {
                        match self.currency {
                            SupportedCurrency::Bitcoin => {
                                "No support for Bitcoin cold signing yet".to_error()?;
                            },
                            SupportedCurrency::Ethereum => {
                                "No support for Ethereum cold signing yet".to_error()?;
                            },
                            c => {
                                format!("Unsupported currency: {} for cold signing", c.json_or()).to_error()?;
                            }
                        }
                    }
                }
            }
        }



        if let Some(updated_tx) = updated.tx.clone() {
            updated.signed_hash = updated_tx.signed_hash().hex();
            updated.ser_tx = Some(updated_tx.json_or());
        }
        Ok(updated.clone())
    }

}
#[async_trait]
impl TxBroadcastProgress for PreparedTransaction {

    async fn broadcast<T>(
        &self,
        mut external_resources: T,
    ) -> RgResult<PreparedTransaction> where T: ExternalNetworkResources + Send {
        let mut updated = self.clone();
        match self.currency.clone() {
            SupportedCurrency::Redgold => {
                let mut tx = self.tx.safe_get_msg("Missing transaction")?.clone();
                updated.broadcast_response = tx.broadcast().await.json_or();
            }
            SupportedCurrency::Bitcoin => {
                updated.broadcast_response = external_resources.send(
                    &self.to, &self.amount, true, Some(self.from.clone()), self.tsi.secret().clone()
                ).await.json_or();
            }
            SupportedCurrency::Ethereum => {
                updated.broadcast_response = external_resources.send(
                    &self.to, &self.amount, true, Some(self.from.clone()), self.tsi.secret().clone()
                ).await.json_or();
            }
            _ => {}
        }
        Ok(updated.clone())
    }

}