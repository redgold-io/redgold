
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};
use redgold_rpc_integ::eth::eth_wallet::EthWalletWrapper;
use redgold_keys::KeyPair;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::util::btc_wallet::SingleKeyBitcoinWallet;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::{error_info, RgResult, SafeOption};
use redgold_schema::structs::{Address, CurrencyAmount, ErrorInfo, Hash, NetworkEnvironment, PublicKey, SupportedCurrency};
use redgold_schema::util::lang_util::AnyPrinter;
use crate::api::client::rest::RgHttpClient;
use crate::core::transact::tx_broadcast_support::TxBroadcastSupport;
use redgold_schema::tx::tx_builder::{TransactionBuilder};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::party::party_events::PartyEvents;
use crate::util;

use core::convert::Infallible;
use std::path::PathBuf;
use std::time::Duration;
use tracing::info;
use tokio::task::JoinHandle;
use redgold_common_no_wasm::retry;
use redgold_common_no_wasm::tx_new::TransactionBuilderSupport;
use redgold_rpc_integ::eth::historical_client::EthHistoricalClient;
use redgold_schema::tx::external_tx::ExternalTimedTransaction;
use crate::core::transact::tx_builder_supports::{TxBuilderApiConvert, TxBuilderApiSupport};
use crate::integrations::external_network_resources::{ExternalNetworkResourcesImpl, MockExternalResources};
// https://stackoverflow.com/questions/75533630/how-to-write-a-retry-function-in-rust-that-involves-async


pub struct PartyTestHarness {
    pub network: NetworkEnvironment,
    pub private_key: String,
    pub keypair: KeyPair,
    pub amm_public_key: PublicKey,
    pub node_config: NodeConfig,
    pub mock_accepted: Vec<Arc<Mutex<HashMap<SupportedCurrency, Vec<ExternalTimedTransaction>>>>>,
    pub last_balance: CurrencyAmount,
    pub client: RgHttpClient,
    pub mock_folders: Vec<PathBuf>,
}

impl PartyTestHarness {

    pub async fn from(
        node_config: &NodeConfig,
        keypair: KeyPair,
        mock_accepted: Vec<Arc<Mutex<HashMap<SupportedCurrency, Vec<ExternalTimedTransaction>>>>>,
        opt_client: Option<RgHttpClient>,
        mock_folders: Vec<std::path::PathBuf>,
    ) -> Self {
        let private_key = keypair.to_private_hex();
        let client = opt_client.unwrap();
        let amm_public_key = client
            .active_party_key().await.unwrap();
        Self {
            network: node_config.network.clone(),
            private_key,
            keypair,
            amm_public_key,
            node_config: node_config.clone(),
            mock_accepted,
            last_balance: CurrencyAmount::zero(SupportedCurrency::Redgold),
            client,
            mock_folders
        }
    }

    pub fn is_mock(&self) -> bool {
        self.mock_accepted.len() > 0 || self.mock_folders.len() > 0
    }

    pub fn client(&self) -> RgHttpClient {
        self.client.clone()
    }

    pub fn self_public(&self) -> PublicKey {
        self.keypair.public_key().clone()
    }

    pub fn self_rdg_address(&self) -> Address {
        self.self_public().address().expect("address")
    }

    pub fn self_btc_address(&self) -> Address {
        self.self_public().to_bitcoin_address_typed(&self.network).expect("address")
    }

    pub fn self_eth_address(&self) -> Address {
        self.self_public().to_ethereum_address_typed().expect("address")
    }

    pub fn amm_btc_address(&self) -> Address {
        self.amm_public_key.to_bitcoin_address_typed(&self.network).expect("address")
    }

    pub fn amm_eth_address(&self) -> Address {
        self.amm_public_key.to_ethereum_address_typed().expect("address")
    }

    pub fn amm_rdg_address(&self) -> Address {
        self.amm_public_key.address().expect("address")
    }

    pub fn btc_swap_amount() -> CurrencyAmount {
        let btc_stake_amt = 15_000;
        let btc_amt = CurrencyAmount::from_btc(btc_stake_amt);
        btc_amt
    }

    pub fn eth_swap_amount() -> CurrencyAmount {
        CurrencyAmount::from_eth_fractional(0.0051)
    }

    pub fn btc_stake_amount() -> CurrencyAmount {
        let btc_stake_amt = 20_000;
        let btc_amt = CurrencyAmount::from_btc(btc_stake_amt);
        btc_amt
    }

    pub fn eth_stake_amount() -> CurrencyAmount {
        CurrencyAmount::stake_test_amount_typed()
    }

    pub fn party_fee_amount() -> CurrencyAmount {
        let party_fee_amount = CurrencyAmount::from_rdg(100000);
        party_fee_amount
    }

    pub async fn tx_builder(&self) -> TransactionBuilder {
        TransactionBuilder::new(&self.node_config)
            .with_input_address(&self.self_rdg_address())
            .clone()
            .into_api_wrapper()
            .with_auto_utxos().await.expect("utxos")
            .clone()
    }

    pub async fn create_external_stake_internal_tx(&self, external_address: &Address, amount: CurrencyAmount) {
        let stake_tx = self.tx_builder().await
            .with_external_stake_usd_bounds(None, None, &self.self_rdg_address(),
                                            external_address, &amount,
                                            &self.amm_rdg_address(), &Self::party_fee_amount())
            .build()
            .expect("build")
            .sign(&self.keypair)
            .expect("sign");
        stake_tx.broadcast_from(&self.node_config).await.expect("broadcast").json_or().print();
    }

    pub async fn send_internal_rdg_stake(&self) -> RgResult<()> {
        let internal_stake_amount = CurrencyAmount::from_fractional(7.0).expect("works");
        let rdg_address = self.self_rdg_address();
        let amm_rdg_address = self.amm_public_key.address().expect("address");
        let internal_stake_tx = self.tx_builder().await
            .with_internal_stake_usd_bounds(
                None, None, &rdg_address, &amm_rdg_address, &internal_stake_amount,
            )
            .build()
            .expect("build")
            .sign(&self.keypair)
            .expect("sign");
        let response = internal_stake_tx
            .broadcast_from(&self.node_config).await.expect("broadcast").json_or();
        println!("response: {response}");
        Ok(())
    }

    // TODO: Change this to use external wallet sender interface.
    pub async fn send_external(&self, amount: CurrencyAmount) {
        let not_mock = !self.is_mock();
        let currency = amount.currency_or();

        let amountu64 = if currency == SupportedCurrency::Ethereum {
            let option = amount.string_amount.clone();
            EthHistoricalClient::translate_value(&option.unwrap()).expect("works")
        } else {
            amount.amount
        };

        let ts = util::current_time_millis_i64();
        let mut mocked = ExternalTimedTransaction {
            tx_id: Hash::from_string_calculate(&*ts.to_string()).hex(),
            timestamp: Some(ts as u64),
            other_address: "".to_string(),
            other_output_addresses: vec![],
            amount: amountu64 as u64,
            bigint_amount: amount.string_amount.clone(),
            incoming: true,
            currency: currency.clone(),
            block_number: Some(0),
            price_usd: None,
            fee: Some(PartyEvents::expected_fee_amount(currency, &self.network).expect("fee")),
            self_address: None,
        };

        match currency {
            SupportedCurrency::Bitcoin => {
                // let mut w =
                //     SingleKeyBitcoinWallet::new_wallet(self.self_public(), self.network.clone(), not_mock)
                //         .expect("w");
                //
                // let dest = self.amm_btc_address();
                //
                // let txid = w.send(&dest, &amount, self.private_key.clone(), not_mock).unwrap();
                mocked.other_address = self.self_btc_address().render_string().unwrap();
            }
            SupportedCurrency::Ethereum => {
                // let w = EthWalletWrapper::new(&self.private_key, &self.network).unwrap();
                // let (txid, _) = w.send_or_form_fake(&self.amm_eth_address(), &amount, &self.keypair, not_mock).await.unwrap();
                // mocked.tx_id = txid;
                mocked.other_address = self.self_eth_address().render_string().unwrap();
            }
            _ => panic!("unsupported currency attempt")
        }
        if self.is_mock() {
            for x in self.mock_accepted.iter() {
                let mut arc = x.lock().await;
                let vec = arc.get_mut(&currency);
                if let Some(v) = vec {
                    v.push(mocked.clone());
                } else {
                    arc.insert(currency.clone(), vec![mocked.clone()]);
                }
            }
            for x in self.mock_folders.iter() {
                if let Ok(ext) = MockExternalResources::new(&self.node_config, Some(x.clone()), Arc::new(Mutex::new(HashMap::new()))) {
                    ext.append_currency_tx(mocked.currency, vec![mocked.clone()]).expect("works");
                }
            }
        }
    }

    pub async fn party_internal_data(&self) -> RgResult<PartyInternalData> {
        self.client().party_data().await?.get(&self.amm_public_key).ok_msg("party data").cloned()
    }

    pub async fn party_events(&self) -> RgResult<PartyEvents> {
        self.party_internal_data().await?.party_events.ok_msg("party events")
    }

    pub async fn verify_internal_stake(&self) -> RgResult<()> {
        let confirmed = self.party_internal_data().await?.party_events.ok_msg("party events")
            ?.internal_staking_events.len() > 0;
        if !confirmed {
            Err(error_info("internal stake not confirmed"))
        } else {
            Ok(())
        }
    }

    pub async fn verify_external_stake_internal_tx(&self) -> RgResult<()> {
        let confirmed = self.party_internal_data().await?.party_events.ok_msg("party events")
            ?.external_staking_events.len() == 2;
        if !confirmed {
            Err(error_info("external stake not confirmed"))
        } else {
            Ok(())
        }
    }

    pub async fn verify_external_stake_internal_tx_1(&self) -> RgResult<()> {
        let confirmed = self.party_internal_data().await?.party_events.ok_msg("party events")
            ?.external_staking_events.len() == 1;
        if !confirmed {
            Err(error_info("external stake not confirmed"))
        } else {
            Ok(())
        }
    }

    pub async fn verify_outgoing_swaps(&self) -> RgResult<()> {
        let length = self.party_internal_data().await?.party_events.ok_msg("party events")
            ?.fulfillment_history.len();
        let confirmed = length == 4;
        info!("Verify outgoing swaps fulfillment_history {}", length);
        if !confirmed {
            Err(error_info("verify_outgoing_swaps failed"))
        } else {
            Ok(())
        }
    }

    pub async fn verify_fulfillment_history(&self, expected_len: usize) -> RgResult<()> {
        let length = self.party_internal_data().await?.party_events.ok_msg("party events")
            ?.fulfillment_history.len();
        let confirmed = length == expected_len;
        info!("Verify verify_fulfillment_historys {} expected {}", length, expected_len);
        if !confirmed {
            Err(error_info(format!("verify fullfillment history len {} failed", expected_len.to_string())))
        } else {
            Ok(())
        }
    }

    pub async fn balance(&mut self, update_self: bool) -> RgResult<CurrencyAmount> {
        let balance = self.client().balance_pk(&self.self_public()).await?;
        info!("balance rdg check: {}", balance.amount);
        if update_self {
            info!("updating self balance");
            self.last_balance = balance.clone();
        }
        Ok(balance)
    }

    pub async fn verify_balance_increased(&mut self) -> RgResult<()> {
        let new_balance = self.balance(false).await?;
        if new_balance.amount <= self.last_balance.amount {
            Err(error_info("balance not increased"))
        } else {
            Ok(())
        }
    }

    pub async fn rdg_to_eth_swap(&self) -> RgResult<()> {
        // test rdg->btc swap
        self.tx_builder().await
            .with_swap(&self.self_eth_address(), &CurrencyAmount::from_fractional(0.4).unwrap(), &self.amm_rdg_address())
            .unwrap()
            .build()
            .unwrap()
            .sign(&self.keypair)
            .unwrap()
            .broadcast_from(&self.node_config).await.expect("broadcast"); //.json_or().print();
        Ok(())
    }

    async fn rdg_to_btc_swap(&self) {
        self.tx_builder().await
            .with_swap(&self.self_btc_address(), &CurrencyAmount::from_fractional(0.4).unwrap(), &self.amm_rdg_address())
            .unwrap()
            .build()
            .unwrap()
            .sign(&self.keypair)
            .unwrap()
            .broadcast_from(&self.node_config).await.expect("broadcast");
    }

    pub async fn check_replicate(&self) -> Option<JoinHandle<()>> {
        let accepted = self.mock_accepted.clone();
        if accepted.len() > 0 {
            Some(tokio::spawn(async move {
                let accepted2 = accepted.clone();
                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    let head = accepted2.get(0).unwrap();
                    let arc = head.lock().await;
                    for (k,v) in arc.iter() {
                        for other in accepted2.iter().skip(1) {
                            let mut arc2 = other.lock().await;
                            let vec = arc2.get_mut(k);
                            if let Some(v2) = vec {
                                for vi in v {
                                    if !v2.contains(vi) {
                                        v2.push(vi.clone());
                                    }
                                }
                            } else {
                                arc2.insert(k.clone(), v.clone());
                            }
                    }
                }
            }
            }))
        } else {
            None
        }
    }

    pub async fn run_test(&mut self) -> RgResult<()> {
        // Spawn thread to check and replicate external TX.

        self.send_internal_rdg_stake().await?;
        retry!(self.verify_internal_stake())?;
        info!("Internal stake confirmed, attempting to stake externally now");
        self.create_external_stake_internal_tx(&self.self_btc_address(), Self::btc_stake_amount()).await;
        self.send_external(Self::btc_stake_amount()).await;
        retry!(self.verify_external_stake_internal_tx_1())?;
        self.create_external_stake_internal_tx(&self.self_eth_address(), Self::eth_stake_amount()).await;
        self.send_external(Self::eth_stake_amount()).await;
        retry!(self.verify_external_stake_internal_tx())?;

        self.swap_post_stake_test().await?;

        Ok(())
    }

    pub async fn swap_post_stake_test(&mut self) -> Result<(), ErrorInfo> {
        info!("Finished with staking, attempting to send swaps now");
        self.balance(true).await?;
        info!("Sending BTC transaction now");
        self.send_external(Self::btc_swap_amount()).await;
        retry!(self.verify_balance_increased())?;
        retry!(async { self.verify_fulfillment_history(1).await })?;
        self.balance(true).await?;
        self.send_external(Self::eth_swap_amount()).await;
        retry!(self.verify_balance_increased())?;
        retry!(async { self.verify_fulfillment_history(2).await })?;
        info!("Finished with deposit swaps, attempting to send internal withdrawal swaps now");
        self.rdg_to_btc_swap().await;
        info!("Send rdg_to_btc_swap deposit swaps");
        retry!(async { self.verify_fulfillment_history(3).await })?;
        self.rdg_to_eth_swap().await.unwrap();
        info!("Sent rdg to eth swap internal tx now awaiting fulfillment externally");
        retry!(async { self.verify_fulfillment_history(4).await })?;
        Ok(())
    }
}
