use futures::{stream, Stream, SinkExt};
use redgold_schema::{ErrorInfoContext, RgResult, SafeOption};
use redgold_schema::structs::ErrorInfo;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::{CurrencyAmount, NetworkEnvironment, SupportedCurrency};
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_tungstenite::{connect_async, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;
use futures::StreamExt;
use url::Url;

#[derive(Clone)]
pub struct BitcoinWsProvider {
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TimestampedBitcoinTransaction {
    pub timestamp: u64,
    pub txid: String,
    pub inputs: Vec<BitcoinInput>,
    pub outputs: Vec<BitcoinOutput>,
    pub block_height: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BitcoinInput {
    pub address: String,
    pub value: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BitcoinOutput {
    pub address: String,
    pub value: u64,
}

impl BitcoinWsProvider {
    pub fn providers_for_network(net: &NetworkEnvironment) -> Vec<String> {
        match net {
            NetworkEnvironment::Main => {
                vec![
                    "wss://ws.blockchain.info/inv",
                    // Add more mainnet providers
                ]
            }
            _ => {
                vec![
                    "wss://ws.blockchain.info/testnet/inv",
                    // Add more testnet providers
                ]
            }
        }.iter().map(|s| s.to_string()).collect()
    }

    pub fn config_to_rpc_urls(config: &NodeConfig) -> Vec<String> {
        config.websocket_rpcs(SupportedCurrency::Bitcoin)
    }

    pub async fn new_from_config(config: &NodeConfig) -> Option<RgResult<BitcoinWsProvider>> {
        let url = Self::config_to_rpc_urls(config).first().cloned();
        match url {
            Some(url) => Some(BitcoinWsProvider::new(url).await),
            None => None
        }
    }

    pub async fn new(url: impl Into<String>) -> RgResult<BitcoinWsProvider> {
        let url = url.into();
        Ok(BitcoinWsProvider { url })
    }

    pub async fn subscribe_transactions(
        &self,
    ) -> RgResult<impl Stream<Item=RgResult<TimestampedBitcoinTransaction>> + '_> {
        let url = Url::parse(&self.url).error_info("Invalid URL")?;
        let (ws_stream, _) = connect_async(url).await.error_info("WebSocket connection failed")?;
        
        // Subscribe to new transactions
        let (mut write, read) = ws_stream.split();
        write.send(Message::Text(r#"{"op":"unconfirmed_sub"}"#.to_string())).await
            .error_info("Failed to subscribe to transactions")?;

        let tx_stream = read.map(move |msg| {
            match msg {
                Ok(Message::Text(text)) => {
                    // Parse the websocket message and convert to TimestampedBitcoinTransaction
                    serde_json::from_str::<TimestampedBitcoinTransaction>(&text)
                        .error_info("JSON parse error")
                }
                Ok(_) => "Unexpected message type".to_error(),
                Err(e) => format!("WebSocket error: {}", e.to_string()).to_error(),
            }
        });

        Ok(tx_stream)
    }

    pub fn convert_transaction(
        party_self_addrs: &Vec<String>,
        timestamped_tx: TimestampedBitcoinTransaction,
    ) -> RgResult<ExternalTimedTransaction> {
        let tx_id = timestamped_tx.txid.clone();
        let ts = timestamped_tx.timestamp;

        // Find the relevant input/output for this transaction
        let mut incoming = true;
        let mut other_address = String::new();
        let mut self_address = String::new();
        let mut amount = 0u64;

        // Check if any of our addresses are in the inputs (sending)
        for input in &timestamped_tx.inputs {
            if party_self_addrs.contains(&input.address) {
                incoming = false;
                self_address = input.address.clone();
                // For outgoing tx, we'll get the "other" address from outputs
                if let Some(first_output) = timestamped_tx.outputs.first() {
                    other_address = first_output.address.clone();
                    amount = first_output.value;
                }
                break;
            }
        }

        // If not found in inputs, check outputs (receiving)
        if incoming {
            let mut found = false;
            for output in &timestamped_tx.outputs {
                if party_self_addrs.contains(&output.address) {
                    self_address = output.address.clone();
                    amount = output.value;
                    // For incoming tx, we'll get the "other" address from inputs
                    if let Some(first_input) = timestamped_tx.inputs.first() {
                        other_address = first_input.address.clone();
                    }
                    found = true;
                    break;
                }
            }
            if !found {
                return "No matching party addrs for tx".to_error()
                    .with_detail("tx", timestamped_tx.json_or())
                    .with_detail("party_self_addrs", party_self_addrs.json_or());
            }
        }

        // Collect all outputs for multi-output support
        let outputs: Vec<(redgold_schema::structs::Address, CurrencyAmount)> = timestamped_tx.outputs
            .iter()
            .map(|o| {
                (
                    redgold_schema::structs::Address::from_bitcoin_external(&o.address),
                    CurrencyAmount::from_btc(o.value as i64)
                )
            })
            .collect();

        Ok(ExternalTimedTransaction {
            tx_id,
            timestamp: Some(ts),
            other_address: other_address.clone(),
            other_output_addresses: vec![],
            amount,
            bigint_amount: Some(amount.to_string()),
            incoming,
            currency: SupportedCurrency::Bitcoin,
            block_number: timestamped_tx.block_height,
            price_usd: None,
            fee: None, // We could calculate this if needed
            self_address: Some(self_address.clone()),
            currency_id: Some(SupportedCurrency::Bitcoin.into()),
            currency_amount: Some(CurrencyAmount::from_btc(amount as i64)),
            from: redgold_schema::structs::Address::from_bitcoin_external(
                &timestamped_tx.inputs.first().map(|i| i.address.clone()).unwrap_or_default()
            ),
            to: outputs,
            other: Some(redgold_schema::structs::Address::from_bitcoin_external(&other_address)),
            queried_address: Some(redgold_schema::structs::Address::from_bitcoin_external(&self_address)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_btc_websocket() {
        let provider = BitcoinWsProvider::new("wss://ws.blockchain.info/inv").await.unwrap();
        let stream = provider.subscribe_transactions().await.unwrap();
        let txs = stream.take(5).collect::<Vec<_>>().await;

        // Test assertions here
        println!("Received {} transactions", txs.len());
        for tx in txs {
            println!("Transaction: {}", tx.json_or());
            let tx = tx.unwrap();
            let party_addr = tx.inputs.first().map(|i| i.address.clone()).unwrap_or_default();
            let ext_tx = BitcoinWsProvider::convert_transaction(&vec![party_addr], tx).unwrap();
            println!("External transaction: {}", ext_tx.json_or());
        }

    }
}
