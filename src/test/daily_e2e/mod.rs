use crate::core::transact::tx_builder_supports::{TxBuilderApiConvert, TxBuilderApiSupport};
use crate::node_config::ApiNodeConfig;
use crate::observability::send_email::email_default;
use crate::test::external_amm_integration::words_to_ci_keypair;
use crate::test::harness::amm_harness::PartyTestHarness;
use log::info;
use redgold_common_no_wasm::retry;
use redgold_common_no_wasm::tx_new::TransactionBuilderSupport;
use redgold_keys::address_external::ToEthereumAddress;
use redgold_keys::KeyPair;
use redgold_rpc_integ::eth::eth_wallet::EthWalletWrapper;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::keys::words_pass::WordsPass;
use redgold_schema::structs::{CurrencyAmount, SupportedCurrency};
use redgold_schema::tx::tx_builder::TransactionBuilder;
use redgold_schema::{RgResult, SafeOption};

pub async fn tx_builder(kp: &KeyPair, nc: &Box<NodeConfig>) -> RgResult<TransactionBuilder> {
    let mut tb = TransactionBuilder::new(&*nc.clone());
    Ok(tb.with_input_address(&kp.address_typed())
        .into_api_wrapper()
        .with_auto_utxos()
        .await?.clone())
}

pub async fn run_daily_e2e(nc: &Box<NodeConfig>) -> RgResult<()> {
    let res = run_daily_e2e_inner(nc).await;
    match res.as_ref() {
        Ok(_) => {
            email_default("Success: Daily E2E Test", "Daily E2E Test Success!").await?;
        }
        Err(e) => {
            email_default("Failure: Daily E2E", e.json_or()).await?;
        }
    }
    res
}
pub async fn run_daily_e2e_inner(nc: &Box<NodeConfig>) -> RgResult<()> {
    let d = nc.config_data.debug.clone().ok_msg("No debug data")?;
    let w = d.words.ok_msg("No words")?;
    let (private, kp) = words_to_ci_keypair(w);
    let api = nc.api_rg_client();
    let party_key = api.active_party_key().await?;
    let party_addr = party_key.address()?;
    let all_party = api.party_data().await?;
    let party = all_party.get(&party_key).ok_msg("No party")?;
    let pev = party.clone().party_events.ok_msg("No party events")?;
    let cpe = pev.central_prices.get(&SupportedCurrency::Ethereum).ok_msg("No eth price")?;

    let w = EthWalletWrapper::new(&private, &nc.network).expect("wallet");
    let mut party_harness = PartyTestHarness::from(
        &nc, kp, vec![], Some(api.clone()), vec![]).await;

    let result = w.send(&party_key.to_ethereum_address_typed().unwrap(), &CurrencyAmount::from_eth_fractional(0.00028914f64), None).await.unwrap();
    info!("Send txid for eth {result}");
    // 60 seconds, 10 times
    let b = party_harness.balance(true).await.unwrap();
    info!("Balance: {}", b.json_or());
    retry!(party_harness.verify_balance_increased(), 120, 20)?;

    let self_pk = &kp.public_key();
    let eth_start_bal = w.get_balance(self_pk).await?;
    info!("Starting eth balance: {}", eth_start_bal.json_or());
    party_harness.rdg_to_eth_swap().await.unwrap();
    retry!(async {
        let bal = w.get_balance(self_pk).await.unwrap();
        if bal > eth_start_bal {
            Ok(())
        } else {
            Err("Balance not increased")
        }
    }, 120, 20).expect("Balance not increased");
    info!("test succeeded");
    // PartyTestHarness::eth_swap_amount()
    // w.send(&party_addr, &CurrencyAmount::from_fractional(0.01)).await?;

    // party_harness.swap_post_stake_test().await?;
    // tb.with_swap(&party_addr, &CurrencyAmount::from_fractional(0.01))

    Ok(())
}