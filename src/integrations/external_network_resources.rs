use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use async_trait::async_trait;
use bdk::bitcoin::psbt::PartiallySignedTransaction;
use bdk::sled::Tree;
use bdk::bitcoin::EcdsaSighashType;
use bdk::database::MemoryDatabase;
use itertools::Itertools;
use tracing::info;
use tokio::sync::Mutex;
use redgold_common::external_resources::{EncodedTransactionPayload, ExternalNetworkResources, NetworkDataFilter};
use redgold_keys::address_external::{get_checksum_address, ToBitcoinAddress, ToEthereumAddress};
use redgold_keys::eth::eth_wallet::EthWalletWrapper;
use redgold_keys::eth::historical_client::EthHistoricalClient;
use redgold_keys::{KeyPair, TestConstants};
use redgold_keys::util::btc_wallet::SingleKeyBitcoinWallet;
use redgold_schema::{error_info, structs, ErrorInfoContext, RgResult, SafeOption};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::structs::{Address, CurrencyAmount, ExternalTransactionId, NetworkEnvironment, PartySigningValidation, Proof, PublicKey, SupportedCurrency};
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use redgold_schema::util::lang_util::AnyPrinter;
use crate::node_config::NodeConfigKeyPair;
use redgold_schema::party::party_events::PartyEvents;
use crate::core::relay::Relay;
use crate::scrape::okx_point;
use crate::test::external_amm_integration::dev_ci_kp;
use crate::util::current_time_millis_i64;

#[derive(Clone)]
pub struct ExternalNetworkResourcesImpl {
    pub btc_wallets: Arc<tokio::sync::Mutex<HashMap<PublicKey, Arc<tokio::sync::Mutex<SingleKeyBitcoinWallet<Tree>>>>>>,
    pub node_config: NodeConfig,
    pub self_secret_key: String,
    pub dummy_secret_key: String,
    pub self_public: PublicKey,
    pub relay: Option<Relay>
}

impl ExternalNetworkResourcesImpl {

    pub fn new(node_config: &NodeConfig, relay: Option<Relay>) -> RgResult<ExternalNetworkResourcesImpl> {
        let btc_wallets = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
        let dummy_secret_key = "25474115328e46e8e636edf6b6f1c90cbd997ae24f5a043fd8ecf2381118e22f".to_string();
        Ok(ExternalNetworkResourcesImpl {
            btc_wallets,
            node_config: node_config.clone(),
            self_secret_key: node_config.keypair().to_private_hex(),
            dummy_secret_key,
            self_public: node_config.keypair().public_key(),
            relay
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
    pub async fn eth_dummy_wallet(&self) -> RgResult<EthWalletWrapper> {
        EthWalletWrapper::new(&self.dummy_secret_key, &self.node_config.network)
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

    async fn broadcast(&mut self, pk: &PublicKey, currency: SupportedCurrency, payload: EncodedTransactionPayload) -> RgResult<String> {
        match currency {
            SupportedCurrency::Bitcoin => {
                let payload = match payload {
                    EncodedTransactionPayload::JsonPayload(s) => s,
                    _ => Err(error_info("Missing payload"))?
                };
                let txid = SingleKeyBitcoinWallet::<MemoryDatabase>::broadcast_tx_static(payload, &self.node_config.network)?;
                Ok(txid)
            },
            SupportedCurrency::Ethereum => {
                let payload = match payload {
                    EncodedTransactionPayload::BytesPayload(vec) => vec,
                    _ => Err(error_info("Missing payload"))?
                };
                let w = EthWalletWrapper::new(&self.dummy_secret_key, &self.node_config.network)?;
                let dec = EthWalletWrapper::decode_rlp_tx(payload.clone())?;
                let txid = dec.hash().to_string();
                w.broadcast_tx_vec(payload).await?;
                Ok(txid)
            }
            _ => Err(error_info("Unsupported currency"))
        }
    }
    async fn query_price(&self, time: i64, currency: SupportedCurrency) -> RgResult<f64> {
        let price = okx_point(time, currency).await?.close;
        Ok(price)
    }

    async fn send(
        &mut self, destination: &Address,
        currency_amount: &CurrencyAmount,
        broadcast: bool,
        from: Option<PublicKey>,
        secret: Option<String>
    ) -> RgResult<(ExternalTransactionId, String)> {
        let secret = secret.unwrap_or_else(|| self.self_secret_key.clone());
        let mut txid = ExternalTransactionId::default();
        txid.currency = currency_amount.currency_or() as i32;
        let mut tx_ser = "".to_string();
        txid.identifier = match currency_amount.currency_or() {
            SupportedCurrency::Bitcoin => {
                let from = from.as_ref().unwrap_or(&self.self_public);
                let arc = self.btc_wallet(from).await?;
                let mut w = arc.lock().await;
                let tx = w.send(&destination, currency_amount, secret, broadcast)?;
                tx_ser = w.psbt.json_or();
                tx
            },
            SupportedCurrency::Ethereum => {
                let w = EthWalletWrapper::new(&secret, &self.node_config.network)?;
                let kp = KeyPair::from_private_hex(secret)?;
                let (txid, ser) = w.send_maybe_broadcast(destination, currency_amount, broadcast).await?;
                tx_ser = ser;
                txid
            }
            _ => return Err(error_info("Unsupported currency"))
        };
        Ok((txid, tx_ser))
    }

    async fn self_balance(&self, currency: SupportedCurrency) -> RgResult<CurrencyAmount> {
        let amount = match currency {
            SupportedCurrency::Bitcoin => {
                let arc = self.btc_wallet(&self.self_public).await?;
                let w = arc.lock().await;
                let raw_balance = w.get_wallet_balance()?.confirmed;
                CurrencyAmount::from_btc(raw_balance as i64)
            },
            SupportedCurrency::Ethereum => {
                let eth = EthHistoricalClient::new(&self.node_config.network).ok_msg("eth client creation")??;
                let eth_addr = self.self_public.to_ethereum_address_typed()?;
                let amount = eth.get_balance_typed(&eth_addr).await?;
                amount
            }
            _ => return Err(error_info("Unsupported currency"))
        };
        Ok(amount)
    }

    async fn btc_payloads(
        &self, outputs: Vec<(String, u64)>, public_key: &PublicKey)
        -> RgResult<(Vec<(Vec<u8>, String)>, PartySigningValidation)> {
        let arc = self.btc_wallet(public_key).await?;
        let mut w = arc.lock().await;
        w.create_transaction_output_batch(outputs)?;

        let pbst_payload = w.psbt.safe_get_msg("Missing PSBT")?.clone().json_or();
        let mut validation = structs::PartySigningValidation::default();
        validation.json_payload = Some(pbst_payload.clone());
        validation.currency = SupportedCurrency::Bitcoin as i32;

        let hashes = w.signable_hashes()?.clone().into_iter().map(|(x,y)|
            (x, y.to_string()))
            .collect_vec();
        Ok((hashes, validation))
    }

    async fn btc_add_signatures(
        &mut self, pk: &PublicKey, psbt: String,
        results: Vec<Proof>, hashes: Vec<(Vec<u8>, String)>) -> RgResult<EncodedTransactionPayload> {
        let psbt = psbt.json_from::<PartiallySignedTransaction>()?;
        let mut w = SingleKeyBitcoinWallet::new_wallet(self.self_public.clone(), self.node_config.network.clone(), false)?;
        // let arc = self.btc_wallet(pk).await?;
        // let mut w = arc.lock().await;
        w.psbt = Some(psbt);
        for (i, ((_, hash_type), result)) in
            hashes.iter().zip(results.iter()).enumerate() {
            let hash_type = EcdsaSighashType::from_str(hash_type).unwrap();
            w.affix_input_signature(i, result, &hash_type);
        }
        w.sign()?;
        Ok(EncodedTransactionPayload::JsonPayload(w.psbt.json_or()))
    }

    async fn eth_tx_payload(&self, src: &Address, dst: &Address, amount: &CurrencyAmount) -> RgResult<(Vec<u8>, PartySigningValidation, String)> {
        let eth = self.eth_dummy_wallet().await?;
        let mut tx = eth.create_transaction_typed(
            &src, &dst, amount.clone(), None
        ).await?;
        let data = EthWalletWrapper::signing_data(&tx)?;
        let tx_ser = tx.json_or();
        let mut valid = structs::PartySigningValidation::default();
        valid.json_payload = Some(tx_ser.clone());
        valid.currency = SupportedCurrency::Ethereum as i32;
        Ok((data, valid, tx_ser))
    }

    async fn max_time_price_by(&self, currency: SupportedCurrency, max_time: i64) -> RgResult<Option<f64>> {
        self.relay.as_ref().unwrap().ds.price_time.max_time_price_by(currency, max_time).await
    }

    async fn get_balance_no_cache(&self, network: &NetworkEnvironment, currency: &SupportedCurrency, pk: &PublicKey)
        -> RgResult<CurrencyAmount> {
        match currency {
            SupportedCurrency::Bitcoin => {
                SingleKeyBitcoinWallet::new_wallet(pk.clone(), network.clone(), false)?.balance()
            }
            SupportedCurrency::Ethereum => {
                self.eth_dummy_wallet().await?.get_balance(pk).await
            }
            _ => "Unsupported currency".to_error()
        }
    }
}


#[derive(Clone)]
pub struct MockExternalResources {
    pub external_transactions: Arc<Mutex<HashMap<SupportedCurrency, Vec<ExternalTimedTransaction>>>>,
    pub inner: ExternalNetworkResourcesImpl,
    pub file_based_prefix: Option<PathBuf>,
    pub node_config: NodeConfig,
    pub dev_ci_kp: KeyPair
}

impl MockExternalResources {

    pub fn new(node_config: &NodeConfig, file_based_prefix: Option<PathBuf>, ext: Arc<Mutex<HashMap<SupportedCurrency, Vec<ExternalTimedTransaction>>>>) -> RgResult<MockExternalResources> {
        let inner = ExternalNetworkResourcesImpl::new(node_config, None)?;
        if let Some(dir) = file_based_prefix.as_ref() {
            std::fs::create_dir_all(dir).error_info("create dir")?;
        }
        Ok(MockExternalResources {
            external_transactions: ext,
            inner,
            file_based_prefix,
            node_config: node_config.clone(),
            dev_ci_kp: dev_ci_kp().unwrap().1,
        })
    }
    pub fn currency_tx_prefix(&self, currency: SupportedCurrency) -> Option<PathBuf> {
        self.file_based_prefix.clone().map(|p| p.join(format!("{:?}", currency)))
    }

    pub fn read_currency_tx(&self, currency: SupportedCurrency) -> RgResult<Vec<ExternalTimedTransaction>> {
        let prefix = self.currency_tx_prefix(currency);
        let path = prefix.ok_or_else(|| error_info("No prefix"))?;
        let contents = std::fs::read_to_string(path).error_info("read to string")?;
        let txs = contents.json_from()?;
        Ok(txs)
    }

    pub fn write_currency_tx(&self, currency: SupportedCurrency, txs: Vec<ExternalTimedTransaction>) -> RgResult<()> {
        let prefix = self.currency_tx_prefix(currency);
        let path = prefix.ok_or_else(|| error_info("No prefix"))?;
        let contents = txs.json_or();
        std::fs::write(path, contents).error_info("write")?;
        Ok(())
    }

    pub fn append_currency_tx(&self, currency: SupportedCurrency, tx: Vec<ExternalTimedTransaction>) -> RgResult<()> {
        let mut txs = self.read_currency_tx(currency).unwrap_or(vec![]);
        txs.extend(tx);
        self.write_currency_tx(currency, txs)
    }

}

#[async_trait]
impl ExternalNetworkResources for MockExternalResources {

    async fn get_all_tx_for_pk(&self, pk: &PublicKey, currency: SupportedCurrency, filter: Option<NetworkDataFilter>)
                               -> RgResult<Vec<ExternalTimedTransaction>> {
        let arc = self.external_transactions.lock().await;
        let option = arc.get(&currency);
        let option1 = option.cloned();
        Ok(option1.unwrap_or_default())
    }

    async fn broadcast(&mut self, pk: &PublicKey, currency: SupportedCurrency, payload: EncodedTransactionPayload) -> RgResult<String> {
        let time = current_time_millis_i64();
        let option = PartyEvents::expected_fee_amount(currency.clone(), &self.inner.node_config.network);
        let expected_fee = option
            .ok_msg("Expected fee missing")?;
        let ett = match currency {
            SupportedCurrency::Bitcoin => {

                let payload = match payload {
                    EncodedTransactionPayload::JsonPayload(s) => s,
                    _ => Err(error_info("Missing payload"))?
                };
                let psbt = payload.json_from::<PartiallySignedTransaction>()?;
                let tx = psbt.extract_tx();
                let time = (time) as u64;
                // let block_time = BlockTime {
                //     height: 0,
                //     timestamp: time,
                // };
                // let det = TransactionDetails{
                //     transaction: Some(tx.clone()),
                //     txid: tx.txid(),
                //     received: 0,
                //     sent: 0,
                //     fee: Some(expected_fee.amount_i64() as u64),
                //     confirmation_time: Some(block_time),
                // };
                let dev_ci = self.dev_ci_kp.public_key().to_bitcoin_address_typed(&self.node_config.network)?.render_string()?;

                let this_btc_addr = pk.to_bitcoin_address_typed(&self.node_config.network)?;
                let this_btc_addr_str = this_btc_addr.render_string()?;
                let outputs = SingleKeyBitcoinWallet::<MemoryDatabase>::outputs_convert_static(&tx.output, self.node_config.network.clone());
                let other_outputs = outputs.iter()
                    .filter(|(ad, am)| ad != &this_btc_addr_str)
                    .filter(|(ad, am)| ad != &dev_ci)
                    .collect_vec();
                let other_output_addresses = other_outputs.iter().map(|(ad, _)| ad.clone()).collect_vec();
                let (other_address, other_amount) = other_outputs.get(0).ok_msg("Missing other output")?.clone().clone();

                // This needs to satisfy multiple 'output' etts
                let ett = ExternalTimedTransaction {
                    tx_id: tx.txid().to_string(),
                    timestamp: Some(time.clone()),
                    other_address,
                    other_output_addresses,
                    amount: other_amount,
                    bigint_amount: None,
                    incoming: false,
                    currency,
                    block_number: Some(0),
                    price_usd: None,
                    fee: Some(expected_fee),
                };
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

                let other_addr = format!("0x{}", hex::encode(tx.to.ok_msg("to missing")?.0));
                let other_addr = get_checksum_address(other_addr);
                ExternalTimedTransaction {
                    tx_id: hex::encode(tx.hash.0),
                    timestamp: Some(time as u64),
                    other_address: other_addr,
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
        info!("External network resource broadcast {}", ett.json_or());
        let mut arc = self.external_transactions.lock().await;
        let existing = arc.get_mut(&currency);
        if let Some(e) = existing {
            info!("Appending to existing txs with len {}" , e.len());
            e.push(ett.clone());
        } else {
            info!("Adding to new txs with len 0");
            arc.insert(currency, vec![ett.clone()]);
        }
        Ok(ett.tx_id.clone())
    }

    async fn query_price(&self, time: i64, currency: SupportedCurrency) -> RgResult<f64> {
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

    async fn send(
        &mut self, destination: &Address, currency_amount: &CurrencyAmount,
        broadcast: bool, from: Option<PublicKey>, secret: Option<String>
    ) -> RgResult<(ExternalTransactionId, String)> {
        let self_pub = self.inner.self_public.clone();
        let self_secret = self.inner.self_secret_key.clone();

        let mut ext = ExternalTransactionId::default();

        match currency_amount.currency_or() {
            SupportedCurrency::Bitcoin => {
                let arc = self.inner.btc_wallet(&self_pub).await?;
                let mut w = arc.lock().await;
                let tx = w.send(destination, currency_amount, self_secret, false)?;
                self.broadcast(&self_pub, SupportedCurrency::Bitcoin, EncodedTransactionPayload::JsonPayload(w.psbt.json_or())).await?;
                ext.currency = SupportedCurrency::Bitcoin as i32;
                ext.identifier = tx;
                let tx_ser = w.psbt.json_or();
                Ok((ext, tx_ser))
            },
            SupportedCurrency::Ethereum => {
                let w = EthWalletWrapper::new(&self_secret, &self.inner.node_config.network)?;
                let (txid, bytes) = w.send_or_form_fake(destination, currency_amount, &self.inner.node_config.keypair(), false).await?;
                let ser = EthWalletWrapper::decode_rlp_tx(bytes.clone())?.json_or();
                self.broadcast(&self_pub, SupportedCurrency::Ethereum, EncodedTransactionPayload::BytesPayload(bytes)).await?;
                ext.currency = SupportedCurrency::Ethereum as i32;
                ext.identifier = txid;
                Ok((ext, ser))
            }
            _ => Err(error_info("Unsupported currency"))
        }
    }

    async fn self_balance(&self, currency: SupportedCurrency) -> RgResult<CurrencyAmount> {
        match currency {
            SupportedCurrency::Bitcoin => {
                let amount = CurrencyAmount::from_btc(100_000_000);
                Ok(amount)
            },
            SupportedCurrency::Ethereum => {
                let amount = CurrencyAmount::from_eth_fractional(100.0);
                Ok(amount)
            }
            _ => Err(error_info("Unsupported currency"))
        }
    }

    async fn btc_payloads(&self, outputs: Vec<(String, u64)>, public_key: &PublicKey) -> RgResult<(Vec<(Vec<u8>, String)>, PartySigningValidation)> { //
        self.inner.btc_payloads(outputs, &self.dev_ci_kp.public_key()).await
    }

    async fn btc_add_signatures(&mut self, pk: &PublicKey, psbt: String, results: Vec<Proof>, hashes: Vec<(Vec<u8>, String)>) -> RgResult<EncodedTransactionPayload> {
        self.inner.btc_add_signatures(&pk, psbt, results, hashes).await
    }

    async fn eth_tx_payload(&self, src: &Address, dst: &Address, amount: &CurrencyAmount) -> RgResult<(Vec<u8>, PartySigningValidation, String)> {
        let eth = self.inner.eth_dummy_wallet().await?;
        let dev_eth_addr = self.dev_ci_kp.public_key().to_ethereum_address_typed().unwrap();
        let mut tx = eth.create_transaction_typed(
            &dev_eth_addr, &dst, amount.clone(), None
        ).await?;
        let data = EthWalletWrapper::signing_data(&tx)?;
        let tx_ser = tx.json_or();
        let mut valid = structs::PartySigningValidation::default();
        valid.json_payload = Some(tx_ser.clone());
        valid.currency = SupportedCurrency::Ethereum as i32;
        Ok((data, valid, tx_ser))
    }

    async fn max_time_price_by(&self, currency: SupportedCurrency, max_time: i64) -> RgResult<Option<f64>> {
        match currency {
            SupportedCurrency::Bitcoin => {
                let price = 60_000.0;
                Ok(Some(price))
            },
            SupportedCurrency::Ethereum => {
                let price = 3_000.0;
                Ok(Some(price))
            }
            _ => Err(error_info("Unsupported currency"))
        }
    }

    async fn get_balance_no_cache(&self, network: &NetworkEnvironment, currency: &SupportedCurrency, pk: &PublicKey) -> RgResult<CurrencyAmount> {
        self.inner.get_balance_no_cache(network, currency, pk).await
    }
}

#[test]
fn generate_dummy_key() {
    let tc = TestConstants::new();
    tc.key_pair().to_private_hex().print();
}