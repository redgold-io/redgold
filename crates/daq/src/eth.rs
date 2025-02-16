use crate::external_net_daq::ExternalDaq;
use async_trait::async_trait;
use futures::future::Either;
use futures::TryStreamExt;
use metrics::counter;
use redgold_common_no_wasm::stream_handlers::IntervalFoldOrReceive;
use redgold_rpc_integ::eth::historical_client::EthHistoricalClient;
use redgold_rpc_integ::eth::ws_rpc::{EthereumWsProvider, TimestampedEthereumTransaction};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::{ErrorInfoContext, RgResult};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::{EnhanceErrorInfo, Loggable};
use redgold_schema::util::lang_util::JsonCombineResult;

#[derive(Clone, Default)]
pub struct EthDaq {
    pub daq: ExternalDaq,
    pub historical_access_api_key: String
}



#[async_trait]
impl IntervalFoldOrReceive<RgResult<TimestampedEthereumTransaction>> for EthDaq {
    async fn interval_fold_or_recv(&mut self, message: Either<RgResult<TimestampedEthereumTransaction>, ()>) -> RgResult<()> {
        match message {
            Either::Left(t) => {
                let t = t?;
                let addrs = self.daq.subscribed_address_filter.read();
                let t_addrs = t.addrs();
                if !t_addrs.iter().any(|a| addrs.contains(a)) {
                    counter!("redgold_daq_eth_skipped_tx").increment(1);
                    return Ok(());
                }
                counter!("redgold_daq_eth_tx").increment(1);
                let ett = EthereumWsProvider::convert_transaction(
                    &addrs, t
                ).log_error();
                if let Ok(ett) = ett {
                    self.daq.add_new_tx(ett.clone());
                }
            }
            _ => {
                for (a, res) in self.daq.historical_transactions.read().iter() {
                    if res.is_err() {
                        self.add_address_and_backfill_historical(a.clone()).await?;
                    }
                }
                // Persist historical here.
            }
        }
        Ok(())
    }
}

impl EthDaq {

    pub async fn add_address_and_backfill_historical(&mut self, address: String) -> RgResult<()> {
        let mut res = self.daq.subscribed_address_filter.clone_read();
        if !res.contains(&address) {
            res.push(address.clone());
            self.daq.subscribed_address_filter.write(res);
        }
        let eth = EthHistoricalClient::new_from_key(
            &self.daq.network, self.historical_access_api_key.clone()
        )?;
        let tx = eth.get_all_tx_with_retries(&address, None, None, None).await;
        self.daq.add_historical_backfill(&address, tx);
        Ok(())
    }

    pub async fn from_eth_provider_stream(
        &self,
        e: RgResult<EthereumWsProvider>,
        interval_duration: Option<Duration>,
        historical_post_fill_duration: Option<Duration>,
    ) -> RgResult<()> {

        let interval_duration = interval_duration.unwrap_or(Duration::from_secs(60));
        let historical_post_fill_duration = historical_post_fill_duration.unwrap_or(Duration::from_secs(60));

        let provider = e?;

        let mut s = self.clone();
        let filter = s.daq.subscribed_address_filter.clone();
        let addrs = filter.read();
        for a in addrs.iter() {
            s.add_address_and_backfill_historical(a.clone()).await?;
        }

        // Run at start
        s.interval_fold_or_recv(Either::Right(())).await?;
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

        tokio::select! {
            res = &mut handle => {
                res.error_info("join error")??;
            }
            _ = tokio::time::sleep(historical_post_fill_duration) => {
                for a in addrs.iter() {
                   s.clone().add_address_and_backfill_historical(a.clone()).await?;
                }
            }
        }
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
        let config_rpcs = EthereumWsProvider::config_to_rpc_urls(nc);
        let fallback_rpcs = EthereumWsProvider::providers_for_network(&nc.network);
        let mut all_rpcs = config_rpcs.clone();
        all_rpcs.extend(fallback_rpcs);
        let mut provider_idx = 0;
        loop {
            let latest_provider = all_rpcs.get(provider_idx).cloned();
            match latest_provider {
                None => { provider_idx = 0 }
                Some(p) => {
                    provider_idx += 1;
                    if let Ok(ws) = EthereumWsProvider::new(p).await.log_error().bubble_abort()? {
                        daq.from_eth_provider_stream(Ok(ws), None, None).await.log_error().bubble_abort()?.ok();
                    };
                }
            }
        }
    }
}

#[ignore]
#[tokio::test]
pub async fn test_eth_daq() {

    let provider = EthereumWsProvider::sepolioa_infura_test().await;
    // let provider = EthereumWsProvider::sepolioa_infura_test().await;
    let daq = EthDaq::default();
    let duration = Duration::from_secs(5);
    // let result = daq.from_eth_provider_stream(Ok(provider), Some(duration), Some(duration)).await.unwrap();
    // tokio::time::sleep(Duration::from_secs(20)).await;


    //
    // let p = EthereumWsProvider::new("ws://server:8556").await.expect("ws provider creation failed");
    // let mut daq = EthDaq::default();
    // let mut eth = EthereumWsProvider::default();
    // let duration = Duration::from_secs(60);
    // let result = daq.from_eth_provider_stream(Ok(eth), duration);
    // assert!(result.is_ok());
}
