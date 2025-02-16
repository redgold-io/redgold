use crate::external_net_daq::ExternalDaq;
use async_trait::async_trait;
use futures::future::Either;
use futures::TryStreamExt;
use metrics::counter;
use redgold_common_no_wasm::stream_handlers::IntervalFoldOrReceive;
use redgold_rpc_integ::btc::ws_rpc::{BitcoinWsProvider, TimestampedBitcoinTransaction};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::{ErrorInfoContext, RgResult};
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::{EnhanceErrorInfo, Loggable};

#[derive(Clone, Default)]
pub struct BtcDaq {
    pub daq: ExternalDaq,
}

#[async_trait]
impl IntervalFoldOrReceive<RgResult<TimestampedBitcoinTransaction>> for BtcDaq {
    async fn interval_fold_or_recv(&mut self, message: Either<RgResult<TimestampedBitcoinTransaction>, ()>) -> RgResult<()> {
        match message {
            Either::Left(t) => {
                let t = t?;
                let addrs = self.daq.subscribed_address_filter.read();
                let t_addrs = t.inputs.iter().map(|i| i.address.clone())
                    .chain(t.outputs.iter().map(|o| o.address.clone()))
                    .collect::<Vec<String>>();
                if !t_addrs.iter().any(|a| addrs.contains(a)) {
                    counter!("redgold_daq_btc_skipped_tx").increment(1);
                    return Ok(());
                }
                counter!("redgold_daq_btc_tx").increment(1);
                let btt = BitcoinWsProvider::convert_transaction(
                    &addrs, t
                ).log_error();
                if let Ok(btt) = btt {
                    self.daq.add_new_tx(btt.clone());
                }
            }
            _ => {
                // TODO: Implement historical backfill if needed
            }
        }
        Ok(())
    }
}

impl BtcDaq {
    pub async fn from_btc_provider_stream(
        &self,
        e: RgResult<BitcoinWsProvider>,
        interval_duration: Option<Duration>,
    ) -> RgResult<()> {
        let interval_duration = interval_duration.unwrap_or(Duration::from_secs(60));
        let provider = e?;

        let mut s = self.clone();
        let s2 = s.clone();

        let mut handle = tokio::spawn(async move {
            let init_stream = provider.subscribe_transactions().await?;
            let stream = init_stream
                .map(|x| Ok(Either::Left(x)));
            let interval = tokio::time::interval(interval_duration);
            let interval_stream = IntervalStream::new(interval).map(|_| Ok(Either::Right(())));
            stream.merge(interval_stream).try_fold(
                s2, |mut ob, o| async {
                    ob.interval_fold_or_recv(o).await.map(|_| ob)
                }
            ).await.map(|_| ())
        });

        handle.await.error_info("Failed to join handle")??;
        Ok(())
    }

    pub async fn start(
        &self,
        nc: &NodeConfig
    ) -> JoinHandle<RgResult<()>> {
        let nc = nc.clone();
        let daq = self.clone();
        tokio::spawn(async move {
            let daq = daq;
            let nc = nc;
            Self::retry_loop(daq, &nc).await
        })
    }

    pub async fn retry_loop(daq: Self, nc: &NodeConfig) -> RgResult<()> {
        let config_rpcs = BitcoinWsProvider::config_to_rpc_urls(nc);
        let fallback_rpcs = BitcoinWsProvider::providers_for_network(&nc.network);
        let mut all_rpcs = config_rpcs.clone();
        all_rpcs.extend(fallback_rpcs);
        let mut provider_idx = 0;
        loop {
            let latest_provider = all_rpcs.get(provider_idx).cloned();
            match latest_provider {
                None => { provider_idx = 0 }
                Some(p) => {
                    provider_idx += 1;
                    if let Ok(ws) = BitcoinWsProvider::new(p).await.log_error().bubble_abort()? {
                        daq.from_btc_provider_stream(Ok(ws), None).await.log_error().bubble_abort()?.ok();
                    };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_btc_daq() {
        let provider = BitcoinWsProvider::new("wss://ws.blockchain.info/inv").await.unwrap();
        let daq = BtcDaq::default();
        let duration = Duration::from_secs(5);
        let _ = daq.from_btc_provider_stream(Ok(provider), Some(duration)).await;
    }
}
