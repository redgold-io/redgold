use redgold_common_no_wasm::tx_new::TransactionBuilderSupport;
use redgold_keys::KeyPair;
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::{RgResult, SafeOption};
use redgold_schema::structs::{CurrencyAmount, SupportedCurrency};
use redgold_schema::tx::tx_builder::TransactionBuilder;
use crate::core::transact::tx_builder_supports::{TxBuilderApiConvert, TxBuilderApiSupport};
use crate::node_config::ApiNodeConfig;
use crate::test::external_amm_integration::words_to_ci_keypair;
use crate::test::harness::amm_harness::PartyTestHarness;

pub async fn tx_builder(kp: &KeyPair, nc: &Box<NodeConfig>) -> RgResult<TransactionBuilder> {
    let mut tb = TransactionBuilder::new(&*nc.clone());
    Ok(tb.with_input_address(&kp.address_typed())
        .into_api_wrapper()
        .with_auto_utxos()
        .await?.clone())
}

pub async fn run_daily_e2e(nc: &Box<NodeConfig>) -> RgResult<()> {
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

    let mut party_harness = PartyTestHarness::from(
        &nc, kp, vec![], Some(api.clone()), vec![]).await;

    party_harness.swap_post_stake_test().await?;
    // tb.with_swap(&party_addr, &CurrencyAmount::from_fractional(0.01))

    Ok(())
}