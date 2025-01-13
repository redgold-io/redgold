use std::time::Duration;
use async_trait::async_trait;
use redgold_common::flume_send_help::{RecvAsyncErrorInfo, SendErrorInfo};
use redgold_schema::RgResult;
use redgold_schema::structs::{RecentDiscoveryTransactionsRequest, Request};
use crate::core::internal_message::TransactionMessage;
use crate::core::relay::Relay;
use redgold_common_no_wasm::stream_handlers::IntervalFold;

pub struct DataDiscovery {
    pub relay: Relay,
}

#[async_trait]
impl IntervalFold for DataDiscovery {
    async fn interval_fold(&mut self) -> RgResult<()> {
        let n = self.relay.trusted_nodes().await?;
        // let n = self.relay.ds.peer_store.active_nodes(None).await?;
        for node in n {
            let mut r = Request::default();
            r.recent_transactions_request = Some(RecentDiscoveryTransactionsRequest{
                limit: None,
                min_time: None
            });
            let res = self.relay.send_message_async(&r, &node, Some(Duration::from_secs(60))).await?;
            let res = res.recv_async_err().await?;
            if let Some(res) = res.recent_discovery_transactions_response {
                for h in res.transaction_hashes {
                    if !self.relay.transaction_known(&h).await? {
                        if self.relay.tx_hash_distance(&h).await? {
                            let t = self.relay.lookup_transaction_serial(&h).await?;
                            if let Some(tx) = t {
                                self.relay.mempool.sender.send_rg_err(
                                     TransactionMessage{
                                         transaction: tx,
                                         response_channel: None,
                                         origin: Some(node.clone()),
                                         origin_ip: None,
                                     }
                                )?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}