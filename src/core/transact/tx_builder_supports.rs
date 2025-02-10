// Really just move the transaction builder to the main thing??

//
// pub trait TransactionBuilderSupportAll {
//
// }
//
// impl TransactionBuilderSupportAll for TransactionBuilder {
//     fn with_ds(&self, ds: DataStore)
// }

use crate::node_config::ApiNodeConfig;
use async_trait::async_trait;
use redgold_schema::tx::tx_builder::TransactionBuilder;
use redgold_schema::RgResult;

#[async_trait]
pub trait TxBuilderApiSupport {
    async fn with_auto_utxos(&mut self) -> RgResult<&mut TransactionBuilder>;

    fn tx_builder(&mut self) -> &mut TransactionBuilder;
}
// Create a newtype wrapper
pub struct TxBuilderApiWrapper(pub TransactionBuilder);

pub trait TxBuilderApiConvert {
    fn into_api_wrapper(&mut self) -> TxBuilderApiWrapper;
}

impl TxBuilderApiConvert for TransactionBuilder {
    fn into_api_wrapper(&mut self) -> TxBuilderApiWrapper {
        TxBuilderApiWrapper(self.clone())
    }
}

#[async_trait]
impl TxBuilderApiSupport for TxBuilderApiWrapper {
    async fn with_auto_utxos(&mut self) -> RgResult<&mut TransactionBuilder> {
        if let Some(nc) = self.0.nc.clone() {
            if self.0.input_addresses.len() > 0 {
                let response = nc.api_client().query_address(self.0.input_addresses.clone()).await?;
                if let Some(qar) = response.query_addresses_response {
                    self.0.with_utxos(&qar.utxo_entries)?;
                }
            }
            if self.0.input_addresses_descriptors.len() > 0 {
                let response = nc.api_client().query_address(
                    self.0.input_addresses_descriptors.iter().map(|x| x.to_address()).collect()
                ).await?;
                if let Some(qar) = response.query_addresses_response {
                    self.0.with_utxos(&qar.utxo_entries)?;
                }
            }
        }
        Ok(&mut self.0)
    }

    fn tx_builder(&mut self) -> &mut TransactionBuilder {
        &mut self.0
    }
}