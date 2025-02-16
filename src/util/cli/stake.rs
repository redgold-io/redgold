use log::info;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_keys::transaction_support::TransactionSupport;
use redgold_keys::word_pass_support::WordsPassNodeConfig;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::conf::rg_args::Stake;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::RgResult;
use redgold_schema::structs::{CurrencyAmount, SupportedCurrency};
use redgold_keys::util::mnemonic_support::MnemonicSupport;
use redgold_schema::SafeOption;
use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};
use crate::integrations::external_network_resources::ExternalNetworkResourcesImpl;
use std::collections::HashMap;
use redgold_schema::helpers::easy_json::EasyJson;
use crate::util;
use crate::node_config::{ApiNodeConfig, ToTransactionBuilder};

pub async fn cli_stake(s: Stake, nc: &Box<NodeConfig>) -> RgResult<()> {
    if s.amount <= 0f64 {
        return "Amount must be greater than 0".to_error();
    }

    let input_currency = s.input_currency.map(|x| {
        SupportedCurrency::try_from(x)
    }).transpose()?.unwrap_or(SupportedCurrency::Redgold);

    if !input_currency.valid_stake_input() {
        return "Invalid input currency".to_error();
    }


    let words = nc.secure_words_or();
    let hot_kp = words.default_kp().unwrap();
    let hot_pk = hot_kp.public_key();
    let hot_addr = hot_pk.address().unwrap();

    info!("Word checksum {}", words.checksum().unwrap());
    info!("Hot keypair RDG address: {}", hot_addr.render_string().unwrap());

    if input_currency == SupportedCurrency::Bitcoin {
        info!("Hot keypair BTC address: {}", hot_pk.to_bitcoin_address_typed(&nc.network).unwrap().render_string().unwrap());
    } else if input_currency == SupportedCurrency::Ethereum {
        info!("Hot keypair ETH address: {}", hot_pk.to_ethereum_address().unwrap());
    }

    let mut res = ExternalNetworkResourcesImpl::new(&nc.clone(), None).unwrap();
    

    let c = nc.api_rg_client();
    let party_key = c.active_party_key().await.unwrap();
    let pd = c.party_data().await.unwrap();
    let pid = pd.get(&party_key).unwrap();

    let pev = pid.party_events.clone().ok_msg("No party events found")?;
    let party_address = pid.metadata.address(&SupportedCurrency::Redgold).unwrap();
    
    if input_currency == SupportedCurrency::Redgold {

        let frac_amt = if s.not_usd {
            s.amount
        } else {
            s.amount / 100.0f64
        };

        let amt = CurrencyAmount::from_fractional(frac_amt).unwrap();

        let mut tb = nc.tx_builder();
        let ai = c.address_info_for_pk(&hot_pk).await.unwrap();
        tb.with_address_info(ai)?;

        // Regular internal stake request
        tb.with_internal_stake_usd_bounds(
            None, 
            None,
            &hot_addr,  // stake control address
            &party_address, // party address
            &amt     // amount to stake
        );

        let mut tx = tb.build().unwrap();
        let signed = tx.sign(&hot_kp).unwrap();
        let result = c.send_transaction(&signed, true).await.unwrap();
        info!("Stake transaction submitted: {}", result.json_or());
    } else {

        let price = res.query_price(util::current_time_millis_i64(), input_currency).await.unwrap();

        // account for not usd
        let amt = if s.not_usd {
            CurrencyAmount::from_fractional_cur(s.amount, input_currency).unwrap()
        } else {
            CurrencyAmount::from_fractional_cur(s.amount / price, input_currency).unwrap()
        };

        // External stake request (BTC/ETH)
        let mut tb = nc.tx_builder();
        let ai = c.address_info_for_pk(&hot_pk).await.unwrap();
        tb.with_address_info(ai)?;

        // Create external stake request
        tb.with_external_stake_usd_bounds(
            None,
            None,
            &hot_addr,  // stake control address
            &hot_addr,  // external address (same as hot_addr since we're using the same keypair)
            &amt,    // external amount
            &party_address, // party address
            &CurrencyAmount::std_pool_fee(), // party fee
        );

        let mut tx = tb.build()?;
        let signed = tx.sign(&hot_kp)?;
        let result = c.send_transaction(&signed, true).await?;
        info!("External stake transaction submitted: {}", result.json_or());

        let res = res.send(&party_address, &amt, true, Some(hot_pk), Some(hot_kp.to_private_hex())).await.unwrap();
        
        info!("External network transaction broadcasted: {}", res.1.json_or());
    }

    Ok(())
}
