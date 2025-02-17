use crate::core::transact::tx_broadcast_support::TxBroadcastSupport;
use crate::test::harness::amm_harness::PartyTestHarness;
use redgold_common_no_wasm::retry;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_schema::structs::{CurrencyAmount, SupportedCurrency};
use redgold_schema::{error_info, RgResult};
use std::collections::HashMap;

impl PartyTestHarness {


    pub async fn create_portfolio(&self) {
        let weights = vec![
            (SupportedCurrency::Bitcoin, 0.5),
            (SupportedCurrency::Ethereum, 0.5)
        ];
        let redgold_to_party_payment_amount = CurrencyAmount::from_fractional(1.01).expect("works");
        self.tx_builder().
            await
            .with_portfolio_request(weights, &redgold_to_party_payment_amount, &self.amm_rdg_address(), &"halfnhalf".to_string())
            .build().unwrap()
            .sign(&self.keypair)
            .unwrap().broadcast_from(&self.node_config).await.unwrap();
    }


    pub async fn create_portfolio_stake_fulfillment_btc(&self, external_amount: CurrencyAmount) {
        let external_amount = CurrencyAmount::from_btc(100_000);
        let redgold_to_party_payment_amount = CurrencyAmount::from_fractional(0.01).expect("works");
        self.tx_builder().
            await
            .with_portfolio_stake_fullfillment(
                &self.self_rdg_address(),
                &self.self_btc_address(),
                &external_amount,
                &self.amm_rdg_address(),
                &redgold_to_party_payment_amount,
            )
            .build().unwrap()
            .sign(&self.keypair)
            .unwrap().broadcast_from(&self.node_config).await.unwrap();
        self.send_external(external_amount).await;
    }

    pub async fn verify_portfolio_request(&self) -> RgResult<()> {
        let is_ok = self.party_events().await.unwrap().portfolio_request_events.events.len() > 0;
        if is_ok {
            Ok(())
        } else {
            Err(error_info("Portfolio request not found"))
        }
    }

    pub async fn verify_stake_internal(&self) -> RgResult<()> {
        let is_ok = self.party_events().await.unwrap().portfolio_request_events.stake_utxos.len() > 0;
        if is_ok {
            Ok(())
        } else {
            Err(error_info("Portfolio request not found"))
        }
    }

    pub async fn portfolio_imbalance(&self) -> HashMap<SupportedCurrency, CurrencyAmount> {
        let events = self.party_events().await.unwrap().portfolio_request_events;
        // info!("Portfolio request events: {}", events.json_or());
        events.current_portfolio_imbalance
    }

    pub async fn run_portfolio_test(&self) {
        self.create_portfolio().await;
        retry!(self.verify_portfolio_request()).unwrap();
        retry!(self.verify_imbalance_initial()).unwrap();
        let imba = self.portfolio_imbalance().await;
        let eth_imbalance = imba.get(&SupportedCurrency::Ethereum).unwrap();
        let btc_imbalance = imba.get(&SupportedCurrency::Bitcoin).unwrap();
        self.create_portfolio_stake_fulfillment_btc(btc_imbalance.clone()).await;
        retry!(self.verify_stake_internal());
        assert!(self.portfolio_imbalance().await.get(&SupportedCurrency::Bitcoin).unwrap().to_fractional() < btc_imbalance.to_fractional());
    }

    async fn verify_imbalance_initial(&self) -> RgResult<()> {
        let imba = self.portfolio_imbalance().await;
        let eth_imbalance = imba.get(&SupportedCurrency::Ethereum).unwrap();
        let btc_imbalance = imba.get(&SupportedCurrency::Bitcoin).unwrap();
        if btc_imbalance.to_fractional() > 0.0 {
            Ok(())
        } else {
            Err(error_info("Initial imbalance not found"))
        }
    }
}