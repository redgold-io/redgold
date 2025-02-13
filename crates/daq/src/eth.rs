use crate::external_net_daq::ExternalDaq;
use async_trait::async_trait;
use futures::future::Either;
use futures::TryStreamExt;
use metrics::counter;
use redgold_common_no_wasm::stream_handlers::IntervalFoldOrReceive;
use redgold_rpc_integ::eth::historical_client::EthHistoricalClient;
use redgold_rpc_integ::eth::ws_rpc::{EthereumWsProvider, TimestampedEthereumTransaction};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::RgResult;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;
use redgold_schema::helpers::easy_json::EasyJson;

#[derive(Clone, Default)]
pub struct EthDaq {
    pub daq: ExternalDaq,
    pub historical_access_api_key: String
}



#[async_trait]
impl IntervalFoldOrReceive<TimestampedEthereumTransaction> for EthDaq {
    async fn interval_fold_or_recv(&mut self, message: Either<TimestampedEthereumTransaction, ()>) -> RgResult<()> {
        match message {
            Either::Left(t) => {
                let addrs = self.daq.subscribed_address_filter.read();
                // let t_addrs = t.addrs();
                // if !t_addrs.iter().any(|a| addrs.contains(a)) {
                //     counter!("redgold_daq_eth_skipped_tx").increment(1);
                //     return Ok(());
                // }
                counter!("redgold_daq_eth_tx").increment(1);
                let ett = EthereumWsProvider::convert_transaction(
                    &addrs, t
                );
                if let Ok(ett) = ett {
                    print!("{}", ett.json_or());
                    if let Some(_) = ett.self_address.as_ref() {
                    }
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
        duration: Duration,
    ) -> RgResult<JoinHandle<RgResult<()>>> {
        let provider = e?;

        let mut s = self.clone();
        let filter = s.daq.subscribed_address_filter.clone();
        let addrs = filter.read();
        for a in addrs.iter() {
            s.add_address_and_backfill_historical(a.clone()).await?;
        }

        // Run at start
        s.interval_fold_or_recv(Either::Right(())).await?;

        Ok(tokio::spawn(async move  {
            let init_stream = provider.subscribe_transactions().await?;
            let stream = init_stream
                .map(|x| Ok(Either::Left(x)));
            let interval = tokio::time::interval(duration);
            let interval_stream = IntervalStream::new(interval).map(|_| Ok(Either::Right(())));
            stream.merge(interval_stream).try_fold(
                s, |mut ob, o| async {
                    ob.interval_fold_or_recv(o).await.map(|_| ob)
                }
            ).await.map(|_| ())
        }))
    }
    pub async fn start(
        &self,
        nc: &NodeConfig
    ) -> Option<RgResult<JoinHandle<RgResult<()>>>> {
        // let duration = nc.config_data.node.as_ref()
        //     .and_then(|n| n.daq.as_ref())
        //     .and_then(|n| n.poll_duration_seconds.clone())
        //     .unwrap_or(600);
        let duration = 60;

        if let Some(p) = EthereumWsProvider::new_from_config(nc).await {
            Some(self.from_eth_provider_stream(p, Duration::from_secs(duration as u64)).await)
        } else {
            None
        }
    }
}

#[ignore]
#[tokio::test]
pub async fn test_eth_daq() {

    let provider = EthereumWsProvider::sepolioa_infura_test().await;
    let daq = EthDaq::default();
    let duration = Duration::from_secs(60);
    let result = daq.from_eth_provider_stream(Ok(provider), duration).await.unwrap();

    tokio::time::sleep(Duration::from_secs(20)).await;


    //
    // let p = EthereumWsProvider::new("ws://server:8556").await.expect("ws provider creation failed");
    // let mut daq = EthDaq::default();
    // let mut eth = EthereumWsProvider::default();
    // let duration = Duration::from_secs(60);
    // let result = daq.from_eth_provider_stream(Ok(eth), duration);
    // assert!(result.is_ok());
}
