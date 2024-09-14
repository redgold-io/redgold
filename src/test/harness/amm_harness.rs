
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};
use redgold_keys::eth::eth_wallet::EthWalletWrapper;
use redgold_keys::KeyPair;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::util::btc_wallet::{ExternalTimedTransaction, SingleKeyBitcoinWallet};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::{error_info, RgResult, SafeOption};
use redgold_schema::structs::{Address, CurrencyAmount, NetworkEnvironment, PublicKey, SupportedCurrency};
use redgold_schema::util::lang_util::AnyPrinter;
use crate::api::RgHttpClient;
use crate::core::transact::tx_broadcast_support::TxBroadcastSupport;
use crate::core::transact::tx_builder_supports::{TransactionBuilder, TransactionBuilderSupport};
use crate::node_config::NodeConfig;
use crate::party::data_enrichment::PartyInternalData;
use crate::party::party_stream::PartyEvents;
use crate::util;

use core::convert::Infallible;
use redgold_keys::eth::historical_client::EthHistoricalClient;

// https://stackoverflow.com/questions/75533630/how-to-write-a-retry-function-in-rust-that-involves-async
#[macro_export]
macro_rules! retry {
    ($f:expr, $count:expr, $interval:expr) => {{
        let mut retries = 0;
        let result = loop {
            let result = $f.await;
            if result.is_ok() {
                break result;
            } else if retries > $count {
                break result;
            } else {
                retries += 1;
                tokio::time::sleep(std::time::Duration::from_secs($interval)).await;
            }
        };
        result
    }};
    ($f:expr) => {
        retry!($f, 10, 100)
    };
}


pub struct PartyTestHarness {
    pub network: NetworkEnvironment,
    pub private_key: String,
    pub keypair: KeyPair,
    pub amm_public_key: PublicKey,
    pub node_config: NodeConfig,
    pub mock_accepted: Vec<Arc<Mutex<HashMap<SupportedCurrency, Vec<ExternalTimedTransaction>>>>>,
    pub last_balance: CurrencyAmount,
    pub client: RgHttpClient
}

impl PartyTestHarness {

    pub async fn from(
        node_config: &NodeConfig,
        keypair: KeyPair,
        mock_accepted: Vec<Arc<Mutex<HashMap<SupportedCurrency, Vec<ExternalTimedTransaction>>>>>,
        opt_client: Option<RgHttpClient>
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
        }
    }

    pub fn is_mock(&self) -> bool {
        self.mock_accepted.len() > 0
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
        EthWalletWrapper::stake_test_amount_typed()
    }

    pub fn party_fee_amount() -> CurrencyAmount {
        let party_fee_amount = CurrencyAmount::from_rdg(100000);
        party_fee_amount
    }

    pub async fn tx_builder(&self) -> TransactionBuilder {
        TransactionBuilder::new(&self.node_config)
            .with_input_address(&self.self_rdg_address())
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
        let internal_stake_tx = TransactionBuilder::new(&self.node_config)
            .with_input_address(&rdg_address)
            .with_auto_utxos().await.expect("utxos")
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

    pub async fn send_external(&self, amount: CurrencyAmount) {
        let not_mock = !self.is_mock();
        let currency = amount.currency_or();

        let amountu64 = if currency == SupportedCurrency::Ethereum {
            let option = amount.string_amount.clone();
            EthHistoricalClient::translate_value(&option.unwrap()).expect("works")
        } else {
            amount.amount
        };

        let mut mocked = ExternalTimedTransaction {
            tx_id: "".to_string(),
            timestamp: Some(util::current_time_millis_i64() as u64),
            other_address: "".to_string(),
            other_output_addresses: vec![],
            amount: amountu64 as u64,
            bigint_amount: amount.string_amount.clone(),
            incoming: true,
            currency: currency.clone(),
            block_number: Some(0),
            price_usd: None,
            fee: Some(PartyEvents::expected_fee_amount(currency, &self.network).expect("fee")),
        };

        match currency {
            SupportedCurrency::Bitcoin => {
                let mut w =
                    SingleKeyBitcoinWallet::new_wallet(self.self_public(), self.network.clone(), not_mock)
                        .expect("w");

                let dest = self.amm_btc_address();

                let txid = w.send(&dest, &amount, self.private_key.clone(), not_mock).unwrap();
                mocked.tx_id = txid;
                mocked.other_address = self.self_btc_address().render_string().unwrap();
            }
            SupportedCurrency::Ethereum => {
                let w = EthWalletWrapper::new(&self.private_key, &self.network).unwrap();
                let txid = w.send_or_form_fake(&self.amm_eth_address(), &amount, &self.keypair, not_mock).await.unwrap();
                mocked.tx_id = txid;
                mocked.other_address = self.self_eth_address().render_string().unwrap();
            }
            _ => panic!("unsupported currency attempt")
        }
        if self.is_mock() {
            for x in self.mock_accepted.iter() {
                let mut arc = x.lock().await;
                let vec = arc.entry(currency.clone()).or_default();
                vec.push(mocked.clone());
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

    pub async fn verify_outgoing_swaps(&self) -> RgResult<()> {
        let confirmed = self.party_internal_data().await?.party_events.ok_msg("party events")
            ?.fulfillment_history.len() == 4;
        if !confirmed {
            Err(error_info("external stake not confirmed"))
        } else {
            Ok(())
        }
    }

    pub async fn balance(&mut self, update_self: bool) -> RgResult<CurrencyAmount> {
        let balance = self.client().balance(&self.self_rdg_address()).await?;
        let amount = CurrencyAmount::from_rdg(balance);
        if update_self {
            self.last_balance = amount.clone();
        }
        Ok(amount)
    }

    pub async fn verify_balance_increased(&mut self) -> RgResult<()> {
        let new_balance = self.balance(false).await?;
        if new_balance.amount <= self.last_balance.amount {
            Err(error_info("balance not increased"))
        } else {
            Ok(())
        }
    }

    pub async fn internal_to_external_swaps(&self) -> RgResult<()> {
        // test rdg->btc swap
        self.tx_builder().await
            .with_swap(&self.self_btc_address(), &CurrencyAmount::from_fractional(0.05551).unwrap(), &self.amm_rdg_address())
            .unwrap()
            .build()
            .unwrap()
            .sign(&self.keypair)
            .unwrap()
            .broadcast_from(&self.node_config).await.expect("broadcast").json_pretty_or().print();
        self.tx_builder().await
            .with_swap(&self.self_eth_address(), &CurrencyAmount::from_fractional(0.05552).unwrap(), &self.amm_rdg_address())
            .unwrap()
            .build()
            .unwrap()
            .sign(&self.keypair)
            .unwrap()
            .broadcast_from(&self.node_config).await.expect("broadcast").json_pretty_or().print();
        Ok(())
    }

    pub async fn run_test(&mut self) -> RgResult<()> {
        self.send_internal_rdg_stake().await?;
        retry!(self.verify_internal_stake())?;
        self.create_external_stake_internal_tx(&self.self_btc_address(), Self::btc_stake_amount()).await;
        self.send_external(Self::btc_stake_amount()).await;
        self.create_external_stake_internal_tx(&self.self_eth_address(), Self::eth_stake_amount()).await;
        self.send_external(Self::eth_stake_amount()).await;
        retry!(self.verify_external_stake_internal_tx())?;
        self.balance(true).await?;
        self.send_external(Self::btc_swap_amount()).await;
        retry!(self.verify_balance_increased())?;
        self.send_external(Self::eth_swap_amount()).await;
        retry!(self.verify_balance_increased())?;
        self.internal_to_external_swaps().await?;
        retry!(self.verify_outgoing_swaps())?;
        Ok(())
    }


}
