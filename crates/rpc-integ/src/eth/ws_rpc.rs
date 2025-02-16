use ethers::middleware::Middleware;
use ethers::prelude::{StreamExt, Transaction};
use ethers::providers::{Provider, Ws};
use ethers::types::U256;
use futures::{stream, Stream};
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::{CurrencyAmount, ErrorInfo, NetworkEnvironment, SupportedCurrency};
use redgold_schema::{structs, ErrorInfoContext, RgResult, SafeOption};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use ethers::prelude::test_provider::TestProvider;
use ethers::providers;

#[derive(Clone)]
pub struct EthereumWsProvider {
    pub provider: Arc<Provider<Ws>>,
    pub url: String
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TimestampedEthereumTransaction {
    pub timestamp: U256,
    pub tx: Transaction
}

impl TimestampedEthereumTransaction {
    pub fn time_ms(&self) -> i64 {
        let ts = BigInt::from_str(&self.timestamp.to_string()).expect("BigInt parse failure");
        ts.to_i64().expect("BigInt to i64 failure") * 1000
    }

    pub fn tx_id(&self) -> String {
        hex::encode(self.tx.hash.0.to_vec())
    }

    pub fn addrs(&self) -> Vec<String> {
        vec![self.tx.from.to_string(), self.tx.to.clone().unwrap_or_default().to_string()]
    }

}

impl EthereumWsProvider {


    pub fn providers_for_network(net: &NetworkEnvironment) -> Vec<String> {
        match net {
            NetworkEnvironment::Main => {
                vec![
                    "wss://eth-mainnet.public.blastapi.io",
                    "wss://mainnet.infura.io/ws/v3/15e8aaed6f894d63a0f6a0206c006cdd", // test keys from ethers
                ]
            }
            _ => {
                vec![
                    "wss://eth-sepolia.public.blastapi.io",
                    "wss://mainnet.infura.io/ws/v3/15e8aaed6f894d63a0f6a0206c006cdd", // test keys from ethers
                ]
            }
        }.iter().map(|s| s.to_string()).collect()
    }

    pub fn config_to_rpc_urls(config: &NodeConfig) -> Vec<String> {
        config.config_data.external.as_ref()
            .and_then(|e| e.rpcs.as_ref())
            .map(|r| r.iter()
                .filter(|r| r.currency == SupportedCurrency::Ethereum)
                .filter(|r| r.url.starts_with("ws"))
                .map(|r| r.url.clone()).collect::<Vec<String>>()
            ).unwrap_or_default()
    }

    pub async fn new_from_config(config: &NodeConfig) -> Option<RgResult<EthereumWsProvider>> {
        let url = Self::config_to_rpc_urls(config).first().cloned();
        match url {
            Some(url) => Some(EthereumWsProvider::new(url).await),
            None => None
        }
    }


    pub async fn new(url: impl Into<String>) -> Result<EthereumWsProvider, ErrorInfo> {
        let url = url.into();
        let provider = Provider::<Ws>::connect(url.clone()).await
            .error_info("Provider ws creation failed")
            .with_detail("url", url.clone())?;
        let provider = Arc::new(provider);
        Ok(EthereumWsProvider {
            provider,
            url
        })
    }

    pub async fn sepolioa_infura_test() -> EthereumWsProvider {
        Self {
            provider: Arc::new(providers::SEPOLIA.ws().await),
            url: "".to_string()
        }
    }


    pub async fn sepolioa_blastapi() -> RgResult<EthereumWsProvider> {
        Self::new("wss://eth-sepolia.public.blastapi.io").await
    }



    pub async fn subscribe_transactions(
        &self
    ) -> RgResult<impl Stream<Item=RgResult<TimestampedEthereumTransaction>> + '_> {
        let block_stream = self.provider
            .subscribe_blocks().await
            .error_info("block subscription failed")?;

        let tx_stream = block_stream.flat_map(move |block| {
            stream::once(async move {
                let block_number = block.number.unwrap_or_default();
                // First get the block with transactions
                let with_tx = match self.provider.get_block_with_txs(block_number).await.error_info("bogus block request") {
                    Ok(block) => block,
                    Err(e) => return vec![Err(e)],
                };

                // Then process the block if we got it
                match with_tx {
                    Some(block) => {
                        // Create timestamped transactions, each wrapped in Ok
                        block.transactions
                            .into_iter()
                            .map(|tx| {
                                Ok(TimestampedEthereumTransaction {
                                    timestamp: block.timestamp,
                                    tx,
                                })
                            })
                            .collect()
                    }
                    None => vec![Err(ErrorInfo::new("Block not found"))]
                }
            })
                .flat_map(stream::iter)
        });

        Ok(tx_stream)
    }

    pub fn convert_transaction(
        party_self_addrs: &Vec<String>,
        timestamped_tx: TimestampedEthereumTransaction
    )
        -> RgResult<ExternalTimedTransaction> {

        let ts = timestamped_tx.time_ms();
        let tx_id = timestamped_tx.tx_id();

        let tx = timestamped_tx.tx;
        let from = tx.from.to_string();
        let to = tx.to.ok_msg("Missing to address")?.to_string();
        let mut incoming = true;
        let mut other_address = to.clone();
        let mut self_address = to.clone();
        if party_self_addrs.contains(&from) {
            incoming = false;
            other_address = from.clone();
            self_address = to.clone();
        } else if !party_self_addrs.contains(&to) {
            return "No matching party addrs for tx".to_error()
                .with_detail("tx", tx.json_or())
                .with_detail("party_self_addrs", party_self_addrs.json_or())
                .with_detail("ts", ts.to_string())
        }
        let g = tx.gas_price.ok_msg("gas price missing")?;
        let g_amount = CurrencyAmount::from_eth_bigint_string((tx.gas * g).to_string());
        let amount = CurrencyAmount::from_eth_bigint_string(tx.value.to_string());

        Ok(ExternalTimedTransaction {
            tx_id,
            timestamp: Some(ts as u64),
            other_address: other_address.clone(),
            other_output_addresses: vec![],
            amount: (amount.to_fractional()*1e8) as u64,
            bigint_amount: Some(tx.value.to_string()),
            incoming,
            currency: SupportedCurrency::Ethereum,
            block_number: tx.block_number.map(|b| b.0[0]),
            price_usd: None,
            fee: Some(g_amount),
            self_address: Some(self_address.clone()),
            currency_id: Some(SupportedCurrency::Ethereum.into()),
            currency_amount: Some(amount.clone()),
            from: structs::Address::from_eth_external_exact(&from),
            to: vec![(structs::Address::from_eth_external_exact(&to), amount)],
            other: Some(structs::Address::from_eth_external_exact(&other_address)),
            queried_address: Some(structs::Address::from_eth_external_exact(&self_address)),
        })
    }
}

use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
#[ignore]
#[tokio::test]
pub async fn ws_stream_test() {
    let p = EthereumWsProvider::new("ws://server:8556").await.expect("ws provider creation failed");
    let s = p.subscribe_transactions().await.unwrap();
    // let mut s = s.take(10).collect::<RgResult<Vec<TimestampedEthereumTransaction>>>().await;
    // println!("Subscribed to new transactions");
    // for tx in s.iter() {
        // println!("Transaction hash: {:?}", tx.hash);
        // println!("From: {:?}", tx.from);
        // println!("To: {:?}", tx.to);
        // println!("Value: {:?}", tx.value);
        // println!("json: {}", tx.json_or());
        // let cbor = tx.to_cbor().unwrap();
        // let from_cbor = Transaction::from_cbor(cbor).unwrap();
        // assert_eq!(tx, &from_cbor);
    // }

    // Serde cbor


    // Use from_samples instead of from_type to infer schema
    // let fields = Vec::<FieldRef>::from_samples(&s, TracingOptions::default()).unwrap();
    
    // Convert to Arrow RecordBatch
    // let batch = serde_arrow::to_record_batch(&fields, &s).unwrap();
    
    // // Write as Parquet
    // let mut writer = ArrowWriter::try_new(output, batch.schema(), None)?;
    // writer.write(&batch)?;
    // writer.close()?;
}