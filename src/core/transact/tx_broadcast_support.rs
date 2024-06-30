use async_trait::async_trait;
use redgold_schema::RgResult;
use redgold_schema::structs::{SubmitTransactionResponse, Transaction};
use crate::node_config::NodeConfig;

#[async_trait]
pub trait TxBroadcastSupport {
    async fn broadcast(&self) -> RgResult<SubmitTransactionResponse>;
}

#[async_trait]
impl TxBroadcastSupport for Transaction {
    async fn broadcast(&self) -> RgResult<SubmitTransactionResponse> {
        let nc = NodeConfig::default_env(self.network()?).await;
        let res = nc.api_client().send_transaction(&self, true).await?;
        Ok(res)
    }
}