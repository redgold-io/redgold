use std::time::Duration;
use crate::external_net_daq::ExternalDaq;
use futures::future::Either;
use metrics::counter;
use redgold_common_no_wasm::arc_swap_wrapper::WriteOneReadAll;
use redgold_common_no_wasm::stream_handlers::{run_interval_fold_or_recv_stream, IntervalFoldOrReceive};
use redgold_rpc_integ::eth::ws_rpc::{EthereumWsProvider, TimestampedEthereumTransaction};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::RgResult;
use tokio::task::JoinHandle;
use redgold_rpc_integ::eth::historical_client::EthHistoricalClient;
use redgold_schema::structs::PublicKey;

#[derive(Clone, Default)]
pub struct EthDaq {
    pub daq: ExternalDaq,
    pub historical_access_api_key: String
}


impl IntervalFoldOrReceive<TimestampedEthereumTransaction> for EthDaq {
    async fn interval_fold_or_recv(&mut self, message: Either<TimestampedEthereumTransaction, ()>) -> RgResult<()> {
        match message {
            Either::Left(t) => {
                let addrs = self.daq.subscribed_address_filter.read();
                let t_addrs = t.addrs();
                if !t_addrs.iter().any(|a| addrs.contains(a)) {
                    counter!("redgold_daq_eth_skipped_tx").increment(1);
                    return Ok(());
                }
                counter!("redgold_daq_eth_tx").increment(1);
                let ett = EthereumWsProvider::convert_transaction(
                    &addrs, t
                );
                if let Ok(ett) = ett {
                    if let Some(a) = ett.self_address.as_ref() {
                    }
                }
            }
            _ => {
                for (a, res) in self.daq.historical_transactions.read().iter() {
                    if res.is_err() {
                        self.add_address_and_backfill_historical(a.clone())?
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
        e: RgResult<EthereumWsProvider>,
        filter: WriteOneReadAll<Vec<String>>,
        duration: Duration,
    ) -> RgResult<JoinHandle<()>> {
        let e = e?;
        let stream = e.subscribe_transactions().await?;
        let mut s = EthDaq::default();
        s.daq.subscribed_address_filter = filter.clone();
        let addrs = filter.read();
        for a in addrs.iter() {
            s.add_address_and_backfill_historical(a.clone()).await?;
        }
        Ok(run_interval_fold_or_recv_stream(s, duration, true, stream))
    }
    pub async fn start(
        nc: &NodeConfig,
        filter: WriteOneReadAll<Vec<String>>
    ) -> Option<RgResult<JoinHandle<()>>> {
        let duration = nc.config_data.node.as_ref()
            .and_then(|n| n.daq.as_ref())
            .and_then(|n| n.poll_duration_seconds.clone())
            .unwrap_or(600);

        if let Some(p) = EthereumWsProvider::new_from_config(nc).await {
            Some(Self::from_eth_provider_stream(p, filter, Duration::from_secs(duration as u64)).await)
        } else {
            None
        }
    }
}
