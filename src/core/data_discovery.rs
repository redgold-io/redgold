use std::time::Duration;
use async_trait::async_trait;
use redgold_schema::RgResult;
use redgold_schema::structs::{Request, RecentDiscoveryTransactionsRequest};
use crate::core::internal_message::{RecvAsyncErrorInfo, SendErrorInfo, TransactionMessage};
use crate::core::relay::Relay;
use crate::core::resolver::resolve_transaction_hash;
use crate::core::stream_handlers::IntervalFold;

pub struct DataDiscovery {
    pub relay: Relay,
}

#[async_trait]
impl IntervalFold for DataDiscovery {
    async fn interval_fold(&mut self) -> RgResult<()> {
        let n = self.relay.ds.peer_store.active_nodes(None).await?;
        for node in n {
            let mut r = Request::default();
            r.recent_transactions_request = Some(RecentDiscoveryTransactionsRequest{
                limit: None,
                min_time: None
            });
            let res = self.relay.send_message_async(&r, &node, Some(Duration::from_secs(5))).await?;
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