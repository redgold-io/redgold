use async_trait::async_trait;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_gui::components::tx_progress::{PreparedTransaction, TransactionProgressFlow};
use redgold_gui::dependencies::gui_depends::TransactionSignInfo;
use redgold_keys::KeyPair;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_schema::{RgResult, SafeOption};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::structs::{PublicKey, SupportedCurrency};
use crate::core::transact::tx_broadcast_support::TxBroadcastSupport;

#[async_trait]
pub trait TxSignerProgress {

    async fn sign<T>(
        &self,
        external_resources: T,
    ) -> RgResult<PreparedTransaction> where T: ExternalNetworkResources + Send ;

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

    async fn sign<T>(
        &self,
        external_resources: T,
    ) -> RgResult<PreparedTransaction>  where T: ExternalNetworkResources + Send {
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
                    TransactionSignInfo::ColdHardwareWallet(h) => {
                        external_resources.trezor_sign(
                            self.from.clone(), h.path, tx.clone()
                        ).await?;
                    }
                    TransactionSignInfo::Qr(q) => {}
                }
            }
            _ => {}
        }


        match self.tsi.clone() {
            TransactionSignInfo::PrivateKey(_) => {
                updated.signed_hash = updated.unsigned_hash.clone();
            }
            TransactionSignInfo::ColdHardwareWallet(h) => {
                let ser = updated.ser_tx.clone().ok_msg("Missing transaction")?;
                match self.currency {
                    SupportedCurrency::Bitcoin => {

                    }
                    SupportedCurrency::Ethereum => {

                    }
                    _ => {}
                }
            }
            TransactionSignInfo::Qr(_) => {}
                // let (txid, tx_ser) = external_resources.send(
                //     &self.to, &self.amount, false, Some(self.from.clone()), self.secret.clone()
                // ).await?;
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