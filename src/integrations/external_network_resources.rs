use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use bdk::bitcoin::psbt::PartiallySignedTransaction;
use bdk::sled::Tree;
use bdk::{BlockTime, TransactionDetails};
use surf::http::headers::ToHeaderValues;
use redgold_data::data_store::DataStore;
use redgold_keys::address_external::ToEthereumAddress;
use redgold_keys::eth::eth_wallet::EthWalletWrapper;
use redgold_keys::eth::historical_client::EthHistoricalClient;
use redgold_keys::TestConstants;
use redgold_keys::util::btc_wallet::{ExternalTimedTransaction, SingleKeyBitcoinWallet};
use redgold_schema::{error_info, RgResult, SafeOption};
use redgold_schema::helpers::easy_json::EasyJsonDeser;
use redgold_schema::structs::{Address, PublicKey, SupportedCurrency};
use redgold_schema::util::lang_util::AnyPrinter;
use crate::node_config::NodeConfig;
use crate::party::party_stream::PartyEvents;
use crate::scrape::okx_point;
use crate::util::current_time_millis_i64;

#[async_trait]
pub trait ExternalNetworkResources {
    async fn get_all_tx_for_pk(&self, pk: &PublicKey, currency: SupportedCurrency, filter: Option<NetworkDataFilter>) -> RgResult<Vec<ExternalTimedTransaction>>;
    async fn broadcast(&mut self, pk: &PublicKey, currency: SupportedCurrency, payload: EncodedTransactionPayload) -> RgResult<()>;
    async fn query_price(&mut self, time: i64, currency: SupportedCurrency) -> RgResult<f64>;

}

pub struct NetworkDataFilter {
    min_block: Option<u64>,
    min_time: Option<u64>
}

pub enum EncodedTransactionPayload {
    JsonPayload(String),
    BytesPayload(Vec<u8>)
}


pub struct ExternalNetworkResourcesImpl {
    pub btc_wallets: Arc<tokio::sync::Mutex<HashMap<PublicKey, Arc<tokio::sync::Mutex<SingleKeyBitcoinWallet<Tree>>>>>>,
    pub node_config: NodeConfig,
    // pub self_secret_key: String,
    pub dummy_secret_key: String
}

impl ExternalNetworkResourcesImpl {

    pub fn new(node_config: &NodeConfig) -> RgResult<ExternalNetworkResourcesImpl> {
        let btc_wallets = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
        let dummy_secret_key = "25474115328e46e8e636edf6b6f1c90cbd997ae24f5a043fd8ecf2381118e22f".to_string();
        Ok(ExternalNetworkResourcesImpl {
            btc_wallets,
            node_config: node_config.clone(),
            dummy_secret_key
        })
    }
    pub async fn btc_wallet(&self, pk: &PublicKey) -> RgResult<Arc<tokio::sync::Mutex<SingleKeyBitcoinWallet<Tree>>>> {
        let mut guard = self.btc_wallets.lock().await;
        let result = guard.get(pk);
        let mutex = match result {
            Some(w) => {
                w.clone()
            }
            None => {
                let new_wallet = SingleKeyBitcoinWallet::new_wallet_db_backed(
                    pk.clone(), self.node_config.network.clone(), true,
                    self.node_config.env_data_folder().bdk_sled_path(),
                    None
                )?;
                let w = Arc::new(tokio::sync::Mutex::new(new_wallet));
                guard.insert(pk.clone(), w.clone());
                w
            }
        };
        Ok(mutex)
    }

}


#[async_trait]
impl ExternalNetworkResources for ExternalNetworkResourcesImpl {
    async fn get_all_tx_for_pk(&self, pk: &PublicKey, currency: SupportedCurrency, filter: Option<NetworkDataFilter>)
                               -> RgResult<Vec<ExternalTimedTransaction>> {
        match currency {
            SupportedCurrency::Bitcoin => {
                let arc = self.btc_wallet(pk).await?;
                let guard = arc.lock().await;
                guard.get_all_tx()
            },
            SupportedCurrency::Ethereum => {
                let eth = EthHistoricalClient::new(&self.node_config.network).ok_msg("eth client creation")??;
                let eth_addr = pk.to_ethereum_address_typed()?;
                let eth_addr_str = eth_addr.render_string()?;

                // Ignoring for now to debug
                let start_block_arg = None;
                // let start_block_arg = start_block;
                let all_tx= eth.get_all_tx_with_retries(&eth_addr_str, start_block_arg, None, None).await?;
                Ok(all_tx)
            }
            _ => Err(error_info("Unsupported currency"))
        }
    }

    async fn broadcast(&mut self, pk: &PublicKey, currency: SupportedCurrency, payload: EncodedTransactionPayload) -> RgResult<()> {
        match currency {
            SupportedCurrency::Bitcoin => {
                let arc = self.btc_wallet(pk).await?;
                let mut w = arc.lock().await;
                let payload = match payload {
                    EncodedTransactionPayload::JsonPayload(s) => s,
                    _ => Err(error_info("Missing payload"))?
                };
                w.psbt = Some(payload.json_from::<PartiallySignedTransaction>()?);
                w.broadcast_tx()
            },
            SupportedCurrency::Ethereum => {
                let payload = match payload {
                    EncodedTransactionPayload::BytesPayload(vec) => vec,
                    _ => Err(error_info("Missing payload"))?
                };
                let w = EthWalletWrapper::new(&self.dummy_secret_key, &self.node_config.network)?;
                w.broadcast_tx_vec(payload).await
            }
            _ => Err(error_info("Unsupported currency"))
        }
    }

    async fn query_price(&mut self, time: i64, currency: SupportedCurrency) -> RgResult<f64> {
        let price = okx_point(time, currency).await?.close;
        Ok(price)
    }
}



pub struct MockExternalResources {
    pub external_transactions: HashMap<SupportedCurrency, Vec<ExternalTimedTransaction>>,
    pub inner: ExternalNetworkResourcesImpl
}

impl MockExternalResources {

    pub fn new(node_config: &NodeConfig) -> RgResult<MockExternalResources> {
        let inner = ExternalNetworkResourcesImpl::new(node_config)?;
        Ok(MockExternalResources {
            external_transactions: HashMap::new(),
            inner
        })
    }
}

#[async_trait]
impl ExternalNetworkResources for MockExternalResources {

    async fn get_all_tx_for_pk(&self, pk: &PublicKey, currency: SupportedCurrency, filter: Option<NetworkDataFilter>)
                               -> RgResult<Vec<ExternalTimedTransaction>> {
        Ok(self.external_transactions.get(&currency).cloned().unwrap_or_default())
    }

    async fn broadcast(&mut self, pk: &PublicKey, currency: SupportedCurrency, payload: EncodedTransactionPayload) -> RgResult<()> {
        let time = current_time_millis_i64();
        let option = PartyEvents::expected_fee_amount(currency.clone(), &self.inner.node_config.network);
        let expected_fee = option
            .ok_msg("Expected fee missing")?;
        let ett = match currency {
            SupportedCurrency::Bitcoin => {
                let arc = self.inner.btc_wallet(pk).await?;
                let mut w = arc.lock().await;
                let payload = match payload {
                    EncodedTransactionPayload::JsonPayload(s) => s,
                    _ => Err(error_info("Missing payload"))?
                };
                let psbt = payload.json_from::<PartiallySignedTransaction>()?;
                let tx = psbt.extract_tx();
                let time = (time / 1000) as u64;
                let block_time = BlockTime {
                    height: 0,
                    timestamp: time,
                };
                let det = TransactionDetails{
                    transaction: Some(tx.clone()),
                    txid: tx.txid(),
                    received: 0,
                    sent: 0,
                    fee: Some(expected_fee.amount_i64() as u64),
                    confirmation_time: Some(block_time),
                };
                let ett = w.extract_ett(&det)?.ok_msg("ett missing")?;
                ett
            },
            SupportedCurrency::Ethereum => {
                // let payload = registered_payload.ok_msg("Missing registered payload")?;
                let payload = match payload {
                    EncodedTransactionPayload::BytesPayload(s) => s,
                    _ => Err(error_info("Missing payload"))?
                };

                let tx = EthWalletWrapper::decode_rlp_tx(payload)?;
                let value_str = tx.value.to_string();
                let amount = EthHistoricalClient::translate_value(&value_str)?;

                ExternalTimedTransaction {
                    tx_id: hex::encode(tx.hash.0),
                    timestamp: Some(time as u64),
                    other_address: tx.to.ok_msg("to missing")?.to_string(),
                    other_output_addresses: vec![],
                    amount: amount as u64,
                    bigint_amount: Some(value_str),
                    incoming: false,
                    currency: SupportedCurrency::Ethereum,
                    block_number: Some(0),
                    price_usd: None,
                    fee: Some(expected_fee.clone()),
                }
            }
            _ => Err(error_info("Unsupported currency"))?
        };
        self.external_transactions.entry(currency).or_insert_with(Vec::new).push(ett);
        Ok(())
    }

    async fn query_price(&mut self, time: i64, currency: SupportedCurrency) -> RgResult<f64> {
        match currency {
            SupportedCurrency::Bitcoin => {
                let price = 60_000.0;
                Ok(price)
            },
            SupportedCurrency::Ethereum => {
                let price = 3_000.0;
                Ok(price)
            }
            _ => Err(error_info("Unsupported currency"))
        }
    }
}




#[test]
fn generate_dummy_key() {
    let tc = TestConstants::new();
    tc.key_pair().to_private_hex().print();
}