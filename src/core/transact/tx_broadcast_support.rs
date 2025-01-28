use crate::node_config::{ApiNodeConfig, EnvDefaultNodeConfig};
use async_trait::async_trait;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::structs::{SubmitTransactionResponse, Transaction};
use redgold_schema::RgResult;

#[async_trait]
pub trait TxBroadcastSupport {
    async fn broadcast(&self) -> RgResult<SubmitTransactionResponse>;

    async fn broadcast_from(&self, nc: &NodeConfig) -> RgResult<SubmitTransactionResponse>;
}

#[async_trait]
impl TxBroadcastSupport for Transaction {
    async fn broadcast(&self) -> RgResult<SubmitTransactionResponse> {
        let nc = NodeConfig::default_env(self.network()?).await;
        self.broadcast_from(&nc).await
    }

    async fn broadcast_from(&self, nc: &NodeConfig) -> RgResult<SubmitTransactionResponse> {
        let res = nc.api_rg_client().send_transaction(&self, true).await?;
        Ok(res)
    }
}