use crate::core::relay::Relay;
use crate::gui::tabs::transact::hardware_signing::gui_trezor_sign;
use crate::scrape::crypto_compare::{crypto_compare_point_query, daily_one_year};
use crate::scrape::okx_point;
use crate::test::external_amm_integration::dev_ci_kp;
use crate::util::current_time_millis_i64;
use async_trait::async_trait;
use bdk::bitcoin::psbt::PartiallySignedTransaction;
use bdk::bitcoin::EcdsaSighashType;
use bdk::database::{BatchDatabase, MemoryDatabase};
use bdk::sled::Tree;
use itertools::Itertools;
use redgold_common::external_resources::{EncodedTransactionPayload, ExternalNetworkResources, NetworkDataFilter, PartyCreationResult, PeerBroadcast};
use redgold_keys::address_external::{get_checksum_address, ToBitcoinAddress, ToEthereumAddress};
use redgold_keys::btc::btc_wallet::SingleKeyBitcoinWallet;
use redgold_keys::word_pass_support::NodeConfigKeyPair;
use redgold_keys::{KeyPair, TestConstants};
use redgold_rpc_integ::eth::eth_wallet::EthWalletWrapper;
use redgold_rpc_integ::eth::historical_client::EthHistoricalClient;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::party::party_events::PartyEvents;
use redgold_schema::structs::{Address, CurrencyAmount, ExternalTransactionId, GetSolanaAddress, Hash, MultisigRequest, MultisigResponse, NetworkEnvironment, PartySigningValidation, Proof, PublicKey, SupportedCurrency, Transaction};
use redgold_schema::message::Response;
use redgold_schema::message::Request;
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use redgold_schema::util::lang_util::AnyPrinter;
use redgold_schema::{bytes_data, error_info, structs, ErrorInfoContext, RgResult, SafeOption, ShortString};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{error, info};
use redgold_common::flume_send_help::SendErrorInfo;
use redgold_common_no_wasm::ssh_like::LocalSSHLike;
use redgold_common_no_wasm::tx_new::TransactionBuilderSupport;
use redgold_keys::address_support::AddressSupport;
use redgold_keys::eth::safe_multisig::SafeMultisig;
use redgold_keys::monero::node_wrapper::MoneroNodeRpcInterfaceWrapper;
use redgold_keys::solana::derive_solana::SolanaWordPassExt;
use redgold_keys::solana::wallet::SolanaNetwork;
use redgold_keys::util::mnemonic_support::MnemonicSupport;
use redgold_node_core::services::monero_wallet_messages::{MoneroSyncInteraction, MoneroWalletMessage, MoneroWalletResponse};
use redgold_schema::hash::ToHashed;
use redgold_schema::keys::words_pass::WordsPass;
use redgold_schema::observability::errors::{EnhanceErrorInfo, Loggable};
use redgold_schema::tx::tx_builder::TransactionBuilder;
use redgold_schema::util::times::ToTimeString;
use crate::core::internal_message::RecvAsyncErrorInfoTimeout;
use crate::core::transact::tx_builder_supports::{TxBuilderApiConvert, TxBuilderApiSupport};
use crate::observability::metrics_help::WithMetrics;
use crate::util;

#[derive(Clone)]
pub struct ExternalNetworkResourcesImpl {
    pub btc_wallets: Arc<tokio::sync::Mutex<HashMap<PublicKey, Arc<tokio::sync::Mutex<SingleKeyBitcoinWallet<Tree>>>>>>,
    pub addr_btc_wallets: Arc<tokio::sync::Mutex<HashMap<Address, Arc<tokio::sync::Mutex<SingleKeyBitcoinWallet<Tree>>>>>>,
    pub multisig_btc_wallets: Arc<tokio::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<SingleKeyBitcoinWallet<Tree>>>>>>,
    pub node_config: Arc<NodeConfig>,
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
            addr_btc_wallets: Arc::new(Default::default()),
            multisig_btc_wallets: Arc::new(Default::default()),
            node_config: Arc::new(node_config.clone()),
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
                    None,
                    None,
                    None,
                    None,
                    None
                )?;
                // println!("New wallet created");
                let w = Arc::new(tokio::sync::Mutex::new(new_wallet));
                guard.insert(pk.clone(), w.clone());
                w
            }
        };
        Ok(mutex)
    }

    pub async fn btc_wallet_for_address(&self, addr: &Address) -> RgResult<Arc<tokio::sync::Mutex<SingleKeyBitcoinWallet<Tree>>>> {
        let mut guard = self.addr_btc_wallets.lock().await;
        let result = guard.get(addr);
        let mutex = match result {
            Some(w) => {
                w.clone()
            }
            None => {
                let buf = self.node_config.env_data_folder().bdk_sled_path();
                let path = buf.join(addr.render_string()?);
                let new_wallet = SingleKeyBitcoinWallet::new_wallet_db_backed(
                    self.self_public.clone(), self.node_config.network.clone(), true,
                    path,
                    None,
                    None,
                    None,
                    None,
                    Some(addr.clone())
                )?;
                // println!("New wallet created");
                let w = Arc::new(tokio::sync::Mutex::new(new_wallet));
                guard.insert(addr.clone(), w.clone());
                w
            }
        };
        Ok(mutex)
    }

    pub async fn btc_multisig_wallet(
        &self,
        peer_pks: &Vec<PublicKey>,
        threshold: i64,
    ) -> RgResult<Arc<tokio::sync::Mutex<SingleKeyBitcoinWallet<Tree>>>> {
        let self_pk = self.self_public.clone();
        let descriptor = SingleKeyBitcoinWallet::<Tree>::multisig_descriptor_create(
            &self_pk, peer_pks, threshold
        )?;
        let mut guard = self.multisig_btc_wallets.lock().await;
        let result = guard.get(&descriptor);
        let mutex = match result {
            Some(w) => {
                w.clone()
            }
            None => {
                let buf = self.node_config.env_data_folder().bdk_sled_path();
                let path = buf.join(descriptor.to_hashed().hex());
                let new_wallet = SingleKeyBitcoinWallet::new_wallet_db_backed(
                    self_pk.clone(),
                    self.node_config.network.clone(),
                    true,
                    path,
                    None,
                    Some(self.self_secret_key.clone()),
                    Some(peer_pks.clone()),
                    Some(threshold),
                    None
                )?;
                // println!("New wallet created");
                let w = Arc::new(tokio::sync::Mutex::new(new_wallet));
                guard.insert(descriptor, w.clone());
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
    fn set_network(&mut self, network: &NetworkEnvironment) {
        let mut nc = (*self.node_config).clone();
        nc.network = network.clone();
        self.node_config = Arc::new(nc);
    }

    async fn execute_external_multisig_send<B: PeerBroadcast>(
        &self,
        destination_amounts: Vec<(Address, CurrencyAmount)>,
        party_address: &Address,
        peer_pks: &Vec<PublicKey>,
        broadcast: &B,
        threshold: i64
    ) -> RgResult<ExternalTransactionId> {
        let (destination, amount) = destination_amounts.get(0).unwrap().clone();
        let currency = destination.currency_or();
        let req = Request::default();
        let mut mreq = MultisigRequest::default();
        mreq.set_currency(currency);
        mreq.amount = Some(amount.clone());
        mreq.destination = Some(destination.clone());
        mreq.mp_address = Some(party_address.clone());
        mreq.proposer_party_key = Some(self.self_public.clone());
        let res = match &currency {
            SupportedCurrency::Bitcoin => {
                let wallet = self.btc_multisig_wallet(
                    peer_pks,
                    threshold
                ).await?;
                let mut guard = wallet.lock().await;
                let result = guard.create_multisig_transaction(destination_amounts, party_address)?;
                let req = Request::default();
                let mut mreq = MultisigRequest::default();
                mreq.set_currency(currency);
                mreq.encoded_tx = Some(result.json_or());
                let results = broadcast.broadcast(peer_pks, req).await?;
                let fixed = results.into_iter().flat_map(|r| r
                    .and_then(|r| r.multisig_response.ok_msg("Missing multisig response"))
                    .and_then(|r| r.encoded_tx.ok_msg("Missing encoded tx"))
                    .and_then(|r| r.json_from::<PartiallySignedTransaction>())
                    .log_error()
                    .with_err_count("redgold_external_party_response_failure").ok()
                ).collect_vec();

                if fixed.len() >= (threshold - 1) as usize {
                    let psbt = guard.combine_psbts(result, fixed)?;
                    let res = guard.finalize_and_broadcast_psbt(psbt)?;
                    res
                } else {
                    "insufficient signatures".to_error()?
                }
            },
            SupportedCurrency::Ethereum => {
                let pkh1 =  self.node_config.keypair().to_private_hex();
                let w1 = EthWalletWrapper::new(&pkh1, &self.node_config.network).unwrap();
                let (tx_hash, bytes) = w1.sign_safe_tx(
                    &party_address,
                    &destination,
                    &amount
                ).await?;

                mreq.bytes_encoded_tx = bytes_data(tx_hash.to_vec());
                let results = broadcast.broadcast(peer_pks, req).await?;
                let fixed = results.into_iter().flat_map(|r| r
                    .and_then(|r| r.multisig_response.ok_msg("Missing multisig response"))
                    .and_then(|r| r.bytes_encoded_tx.ok_msg("Missing encoded tx"))
                    .map(|r| r.value)
                    .log_error()
                    .with_err_count("redgold_external_party_response_failure").ok()
                ).collect_vec();

                let mut all = vec![bytes.to_vec()];
                all.extend(fixed);

                w1.execute_safe_transaction(&party_address, &destination, &amount, all, tx_hash).await?
            }
            _ => Err(error_info("Unsupported currency for multisig"))?
    };
    Ok(ExternalTransactionId{
        identifier: res,
        currency: currency as i32,
    })
}

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
                if let Some(r) = self.relay.as_ref() {
                    let all_tx2_comparison = r.eth_daq.daq.all_tx_for(&eth_addr_str);
                    let missing_hashes = all_tx.iter().filter(|tx| {
                        !all_tx2_comparison.iter().any(|tx2| tx2.tx_id == tx.tx_id)
                    }).map(|tx| tx.tx_id.clone()).collect_vec();
                    let equality = all_tx.iter().zip(all_tx2_comparison.iter()).all(|(tx1, tx2)| tx1 == tx2);
                    info!("EthDaq all tx comparison: {} vs {} equality: {} missing hashes: {:?}", all_tx.len(), all_tx2_comparison.len(), equality, missing_hashes);
                }

                Ok(all_tx)
            }
            _ => Err(error_info("Unsupported currency"))
        }
    }

    async fn get_all_tx_for_address(&self, address: &Address, currency: SupportedCurrency, filter: Option<NetworkDataFilter>)
                               -> RgResult<Vec<ExternalTimedTransaction>> {
        match currency {
            SupportedCurrency::Bitcoin => {
                let arc = self.btc_wallet_for_address(address).await?;
                let guard = arc.lock().await;
                guard.get_all_tx()
            },
            SupportedCurrency::Ethereum => {
                let eth = EthHistoricalClient::new(&self.node_config.network).ok_msg("eth client creation")??;
                let eth_addr = address.clone();
                let eth_addr_str = eth_addr.render_string()?;

                // Ignoring for now to debug
                let start_block_arg = None;
                // let start_block_arg = start_block;
                let all_tx= eth.get_all_tx_with_retries(&eth_addr_str, start_block_arg, None, None).await?;
                if let Some(r) = self.relay.as_ref() {
                    let all_tx2_comparison = r.eth_daq.daq.all_tx_for(&eth_addr_str);
                    let missing_hashes = all_tx.iter().filter(|tx| {
                        !all_tx2_comparison.iter().any(|tx2| tx2.tx_id == tx.tx_id)
                    }).map(|tx| tx.tx_id.clone()).collect_vec();
                    let equality = all_tx.iter().zip(all_tx2_comparison.iter()).all(|(tx1, tx2)| tx1 == tx2);
                    info!("EthDaq all tx comparison: {} vs {} equality: {} missing hashes: {:?}", all_tx.len(), all_tx2_comparison.len(), equality, missing_hashes);
                }
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
        if currency == SupportedCurrency::Monero {
            return crypto_compare_point_query(currency, time).await
        };
        let price = okx_point(time, currency).await?.close;
        Ok(price)
    }

    async fn daily_historical_year(&self) -> RgResult<HashMap<SupportedCurrency, Vec<(i64, f64)>>> {
        daily_one_year().await
    }

    async fn send(
        &mut self,
        destination: &Address,
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

    async fn eth_tx_payload(&self, src: &Address, dst: &Address, amount: &CurrencyAmount, override_gas: Option<CurrencyAmount>) -> RgResult<(Vec<u8>, PartySigningValidation, String)> {
        let eth = self.eth_dummy_wallet().await?;
        let tx = eth.create_transaction_typed(
            &src, &dst, amount.clone(), override_gas
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

    async fn trezor_sign(&self, public: PublicKey, derivation_path: String, t: Transaction) -> RgResult<Transaction> {
        let mut t = t.clone();
        gui_trezor_sign(public, derivation_path, &mut t).await
    }


    async fn get_live_balance(&self, address: &Address) -> RgResult<CurrencyAmount> {
        match address.currency_or() {
            SupportedCurrency::Bitcoin => {
                let arc = self.btc_wallet_for_address(address).await?;
                let w = arc.lock().await;
                w.balance()
            },
            SupportedCurrency::Redgold => {
                self.relay.safe_get_msg("Missing relay")?.ds.utxo
                    .get_balance_for_addresses(vec![address.clone()]).await
            },
            SupportedCurrency::Ethereum => {
                let eth = EthHistoricalClient::new(&self.node_config.network).ok_msg("eth client creation")??;
                let eth_addr = address.clone();
                let amount = eth.get_balance_typed(&eth_addr).await?;
                Ok(amount)
            }
            SupportedCurrency::Solana => {
                let sol = SolanaNetwork::new(self.node_config.network.clone(), None);
                let balance = sol.get_balance(address.clone()).await?;
                Ok(balance)
            },
            _ => Err(error_info("Unsupported currency"))
        }
    }

    async fn btc_pubkeys_to_multisig_address(&self, peer_keys: &Vec<PublicKey>, thresh: i64) -> RgResult<Address> {
        let arc = self.btc_multisig_wallet(
            peer_keys, thresh
        ).await?;
        let w = arc.lock().await;
        w.get_descriptor_address_typed()
    }

    async fn create_multisig_party<B: PeerBroadcast + Sync>(
        &self,
        cur: &SupportedCurrency,
        all_pks: &Vec<PublicKey>,
        self_public_key: &PublicKey,
        self_private_key_hex: &String,
        network: &NetworkEnvironment,
        words_pass: WordsPass,
        threshold: i64,
        peer_broadcast: &B,
        peer_pks: &Vec<PublicKey>
    ) -> RgResult<Option<PartyCreationResult>> {

        let mut secret_json = None;

        let res = match cur.clone() {
            SupportedCurrency::Redgold => {
                let address = Address::from_multisig_public_keys_and_threshold(&all_pks, threshold);
                info!("Created redgold multisig address @ {}", address.json_or());
                Some(address)
            }
            SupportedCurrency::Bitcoin => {
                Some(self.btc_pubkeys_to_multisig_address(&peer_pks, threshold).await?)
            }
            SupportedCurrency::Ethereum => {
                let self_address = self_public_key.to_ethereum_address_typed()?;
                let safe = SafeMultisig::new(network.clone(), self_address, self_private_key_hex.clone());
                let owners = all_pks.iter()
                    .map(|pk| pk.to_ethereum_address_typed())
                    .collect::<RgResult<Vec<Address>>>()?;
                let creation_result = safe.create_safe(threshold, owners).await?;
                info!("Created safe ethereum multisig contract @ {:?}", creation_result);
                Some(creation_result.safe_addr.parse_ethereum_address_external()?)
            }
            SupportedCurrency::Solana => {
                let sol = SolanaNetwork::new(network.clone(), Some(words_pass.clone()));
                let mut req = Request::default();
                let gs = GetSolanaAddress::default();
                req.get_solana_address = Some(gs);
                let responses = peer_broadcast.broadcast(peer_pks, req).await;
                let mut addrs = responses?
                    .into_iter().map(|r| {
                    r.and_then(|resp| { resp.clone().with_error_info().clone()})
                        .and_then(|r|
                            r.get_solana_address_response.clone().ok_msg("Missing solana address response")
                        )
                }).collect::<RgResult<Vec<Address>>>()?;

                let self_addr = words_pass.solana_address()?;
                addrs.push(self_addr);

                let res = sol.establish_multisig_party(addrs, threshold).await?;
                Some(Address::from_solana_external(&res))
            }
            SupportedCurrency::Monero => {

                let (s, r) =
                    flume::bounded::<RgResult<MoneroWalletResponse>>(1);
                self.relay.safe_get_msg("Missing relay")?.monero_wallet_messages
                    .sender.send_rg_err(MoneroSyncInteraction{
                    message: MoneroWalletMessage::MultisigCreateNext,
                    wallet_id: MoneroNodeRpcInterfaceWrapper::<LocalSSHLike>::get_wallet_filename_id(
                        all_pks, threshold
                    ),
                    all_pks: all_pks.clone(),
                    peer_strings: vec![],
                    threshold,
                    response: s,
                    operation_initialization: true,
                })?;
                let result = r.recv_async_err_timeout(Duration::from_secs(120)).await??;
                let mw = match result {
                    MoneroWalletResponse::PeerCreate(_) => { None}
                    MoneroWalletResponse::InstanceCreate(i) => {
                        secret_json = Some(i.json_or());
                        Some(i.address)
                    }
                };
                mw
            }
            _ => {
                error!("Unsupported currency for party formation: {:?}", cur);
                Err(error_info("Unsupported currency"))?
            }
        };
        Ok(res.map(|a| PartyCreationResult {
            address: a,
            secret_json
        }))
    }

    async fn participate_multisig_send(
        &self,
        mr: MultisigRequest,
        peer_pks: &Vec<PublicKey>,
        threshold: i64
    ) -> RgResult<MultisigResponse> {
        let dest = mr.destination.safe_get_msg("Missing destination")?;
        let party = mr.mp_address.safe_get_msg("Missing party address")?;
        let amount = mr.amount.safe_get_msg("Missing amount")?;
        match dest.currency_or() {
            SupportedCurrency::Bitcoin => {
                let addr = self.btc_multisig_wallet(
                    &peer_pks, threshold
                ).await?;
                let mut guard = addr.lock().await;
                let psbt = mr.encoded_tx.safe_get_msg("Missing encoded tx")?;
                let psbt = psbt.json_from::<PartiallySignedTransaction>()?;
                let result = guard.sign_multisig_psbt(psbt)?;
                Ok(MultisigResponse{
                    tx: None,
                    encoded_tx: Some(result.json_or()),
                    bytes_encoded_tx: None,
                    currency: dest.currency_or() as i32,
                })
            }
            SupportedCurrency::Ethereum => {
                let eth = EthWalletWrapper::new(&self.self_secret_key, &self.node_config.network)?;
                let bytes = mr.bytes_encoded_tx.safe_get_msg("Missing encoded tx")?;
                let bytes = bytes.value.clone();
                let (hash, sigbytes) = eth.sign_safe_tx(&party, dest, amount).await?;
                if hash.to_vec() != bytes {
                    error!("Hash mismatch: {:?} vs {:?}", hash, bytes);
                }
                Ok(MultisigResponse{
                    tx: None,
                    encoded_tx: None,
                    bytes_encoded_tx: bytes_data(sigbytes.to_vec()),
                    currency: dest.currency_or() as i32,
                })
            }
                _ => Err(error_info("Unsupported currency"))?
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
    fn set_network(&mut self, network: &NetworkEnvironment) {
        self.node_config.network = network.clone();
    }

    async fn get_all_tx_for_address(&self, address: &Address, currency: SupportedCurrency, filter: Option<NetworkDataFilter>) -> RgResult<Vec<ExternalTimedTransaction>> {
        let arc = self.external_transactions.lock().await;
        let option = arc.get(&currency);
        let option1 = option.cloned().into_iter().flat_map(|x|
        x.into_iter().filter(|y| y.queried_address.as_ref() == Some(address))
        ).collect_vec();
        Ok(option1)
    }

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
                    other_address: other_address.clone(),
                    other_output_addresses,
                    amount: other_amount,
                    bigint_amount: None,
                    incoming: false,
                    currency,
                    block_number: Some(0),
                    price_usd: None,
                    fee: Some(expected_fee),
                    self_address: None,
                    currency_id: Some(SupportedCurrency::Bitcoin.into()),
                    currency_amount: Some(CurrencyAmount::from_btc(other_amount as i64)),
                    from: this_btc_addr.clone(),
                    to: other_outputs.iter().map(|(ad, amt)|
                        (Address::from_bitcoin_external(ad), CurrencyAmount::from_btc(amt.clone() as i64))).collect_vec(),
                    other: Some(Address::from_bitcoin_external(&other_address)),
                    queried_address: Some(this_btc_addr.clone()),
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
                    other_address: other_addr.clone(),
                    other_output_addresses: vec![],
                    amount: amount as u64,
                    bigint_amount: Some(value_str.clone()),
                    incoming: false,
                    currency: SupportedCurrency::Ethereum,
                    block_number: Some(0),
                    price_usd: None,
                    fee: Some(expected_fee.clone()),
                    self_address: None,
                    currency_id: Some(SupportedCurrency::Ethereum.into()),
                    currency_amount: Some(CurrencyAmount::from_eth_bigint_string(value_str.clone())),
                    from: Address::from_eth_external_exact(&tx.from.to_string()),
                    to: vec![(Address::from_eth_external_exact(&other_addr.clone()), CurrencyAmount::from_eth_bigint_string(value_str))],
                    other: Some(Address::from_eth_external_exact(&other_addr)),
                    queried_address: Some(Address::from_eth_external_exact(&tx.from.to_string())),
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
        let res = match currency {
            SupportedCurrency::Bitcoin => 100_000.0,
            SupportedCurrency::Ethereum => 3_000.0,
            SupportedCurrency::Monero => 200.0,
            SupportedCurrency::Solana => 200.0,
            SupportedCurrency::Redgold => 100.0,
            _ => "Unsupported currency for query price".to_error()
                .with_detail("currency", format!("{:?}", currency))
                .with_detail("time", time.to_time_string_shorter_underscores())?
        };
        Ok(res)
    }

    async fn daily_historical_year(&self) -> RgResult<HashMap<SupportedCurrency, Vec<(i64, f64)>>> {
        "not implemented".to_error()
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

    async fn eth_tx_payload(&self, src: &Address, dst: &Address, amount: &CurrencyAmount, override_gas: Option<CurrencyAmount>) -> RgResult<(Vec<u8>, PartySigningValidation, String)> {
        let eth = self.inner.eth_dummy_wallet().await?;
        let dev_eth_addr = self.dev_ci_kp.public_key().to_ethereum_address_typed().unwrap();
        let tx = eth.create_transaction_typed(
            &dev_eth_addr, &dst, amount.clone(), override_gas
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

    async fn trezor_sign(&self, public: PublicKey, derivation_path: String, t: Transaction) -> RgResult<Transaction> {
        "Not implemented".to_error()
    }


    async fn get_live_balance(&self, address: &Address) -> RgResult<CurrencyAmount> {
        let cur = address.currency_or();
        let usd_price = cur.price_default();
        let amount = 1000f64 / usd_price;
        CurrencyAmount::from_fractional_cur(amount, cur)
    }

    async fn btc_pubkeys_to_multisig_address(&self, pubkeys: &Vec<PublicKey>, thresh: i64) -> RgResult<Address> {
        pubkeys.get(0).clone().ok_msg("btc")?.to_bitcoin_address_typed(&self.node_config.network)
    }

    async fn create_multisig_party<B: PeerBroadcast>(
        &self,
        cur: &SupportedCurrency,
        all_pks: &Vec<PublicKey>,
        self_public_key: &PublicKey,
        self_private_key_hex: &String,
        network: &NetworkEnvironment,
        words_pass: WordsPass,
        threshold: i64,
        peer_broadcast: &B,
        peer_pks: &Vec<PublicKey>
    ) -> RgResult<Option<PartyCreationResult>> {
        let tw = TestConstants::new().words_pass;
        let addrs = tw.to_all_addresses_default(&network)?;
        let addr = addrs.iter().filter(|a| &a.currency_or() == cur)
            .next()
            .ok_msg("Missing currency")?;
        addr.as_external();
        let mut result = PartyCreationResult::default();
        result.address = addr.clone();
        match cur {
            SupportedCurrency::Redgold => {
                result.address = Address::from_multisig_public_keys_and_threshold(all_pks, threshold)
            }
            _ => {
            }
        }
        Ok(Some(result))
    }

    async fn execute_external_multisig_send<B: PeerBroadcast>(
        &self,
        destination_amounts: Vec<(Address, CurrencyAmount)>,
        party_address: &Address,
        peer_pks: &Vec<PublicKey>,
        broadcast: &B,
        threshold: i64
    ) -> RgResult<ExternalTransactionId> {
        let currency = party_address.currency_or();
        let (destination, amount) = destination_amounts.get(0).cloned().unwrap();
        let amountu64 = (amount.to_fractional() * 1e8) as u64;
        // TODO: May need to fill out other fields here?
        let ts = util::current_time_millis_i64();
        let mut mocked = ExternalTimedTransaction {
            tx_id: Hash::from_string_calculate(&*ts.to_string()).hex(),
            timestamp: Some(ts as u64),
            other_address: destination.render_string().unwrap(),
            other_output_addresses: vec![],
            amount: amountu64,
            bigint_amount: amount.string_amount.clone(),
            incoming: false,
            currency: currency.clone(),
            block_number: Some(0),
            price_usd: None,
            fee: Some(PartyEvents::expected_fee_amount(currency, &self.node_config.network).expect("fee")),
            self_address: Some(party_address.render_string().unwrap()),
            currency_id: Some(currency.to_currency_id()),
            currency_amount: Some(amount.clone()),
            from: party_address.clone(),
            to: vec![(destination.clone(), amount.clone())],
            other: Some(destination.clone()),
            queried_address: Some(party_address.clone()),
        };

        let mut arc = self.external_transactions.lock().await;
        let vec = arc.get_mut(&currency);
        if let Some(v) = vec {
            v.push(mocked.clone());
        } else {
            arc.insert(currency.clone(), vec![mocked.clone()]);
        }
        Ok(ExternalTransactionId{
            identifier: mocked.tx_id.clone(),
            currency: currency as i32,
        })
    }

    async fn participate_multisig_send(&self, mr: MultisigRequest, peer_pks: &Vec<PublicKey>, threshold: i64) -> RgResult<MultisigResponse> {
        Ok(MultisigResponse::default())
    }
}

#[test]
fn generate_dummy_key() {
    let tc = TestConstants::new();
    tc.key_pair().to_private_hex().print();
}