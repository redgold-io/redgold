use std::collections::HashMap;
use log::info;
use redgold_common::external_resources::ExternalNetworkResources;
use redgold_common::flume_send_help::Channel;
use redgold_gui::state::local_state::LocalStateUpdate;
use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};
use redgold_keys::util::mnemonic_support::MnemonicSupport;
use redgold_keys::word_pass_support::WordsPassNodeConfig;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::conf::rg_args::Swap;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::keys::words_pass::WordsPass;
use redgold_schema::{RgResult, SafeOption};
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::structs::{CurrencyAmount, SupportedCurrency};
use crate::gui::components::tx_signer::{TxBroadcastProgress, TxSignerProgress};
use crate::gui::ls_ext::create_swap_tx;
use crate::gui::native_gui_dependencies::NativeGuiDepends;
use crate::integrations::external_network_resources::ExternalNetworkResourcesImpl;
use crate::node_config::ApiNodeConfig;
use crate::party::portfolio_request::PortfolioEventMethods;
use crate::util;

pub async fn cli_swap(s: Swap, nc: &Box<NodeConfig>) -> RgResult<()> {

    if s.amount <= 0f64 {
        return "Amount must be greater than 0".to_error();
    }
    if !s.input_currency.valid_swap_input() {
        return "Invalid input currency".to_error();
    }
    if !s.output_currency.valid_swap_output() {
        return "Invalid output currency".to_error();
    }




    let words = nc.secure_words_or();
    let hot_kp = words.default_kp().unwrap();
    let hot_pk = hot_kp.public_key();
    let hot_addr = hot_pk.address().unwrap();

    info!("Word checksum {}", words.checksum().unwrap());
    info!("Hot keypair loaded @ derivation path: {}", WordsPass::default_rg_path(0));
    info!("Hot keypair RDG address: {}", hot_addr.render_string().unwrap());
    let btc_addr =  hot_pk.to_bitcoin_address_typed(&nc.network).unwrap().render_string().unwrap();
    info!("Hot keypair BTC address: {}", btc_addr);
    info!("Hot keypair ETH address: {}", hot_pk.to_ethereum_address().unwrap());
    let res = ExternalNetworkResourcesImpl::new(&nc.clone(), None).unwrap();
    info!("Getting external prices");

    let mut price_map = HashMap::new();
    for cur in SupportedCurrency::supported_external_swap_currencies() {
        let p = res.query_price(util::current_time_millis_i64(), cur.clone()).await?;
        info!("Price USD for {}: {}", cur.to_display_string(), p);
        price_map.insert(cur, p);
    }


    let mut bm = HashMap::new();
    info!("Getting external balances");
    for cur in SupportedCurrency::supported_external_swap_currencies() {
        let b = res.self_balance(cur.clone()).await?;
        info!("Balance for {}: {} {} -- ${:.2} USD", cur.to_display_string(), b.to_fractional(), cur.abbreviated(),
            (b.to_fractional() * price_map.get(&cur).unwrap()));
        bm.insert(cur, b);
    }


    let c = nc.api_rg_client();

    info!("Getting party data");

    let g = NativeGuiDepends::new(*nc.clone());
    let party_key = c.active_party_key().await.unwrap();
    let pd = c.party_data().await.unwrap();
    let pid = pd.get(&party_key).unwrap();

    let pev = pid.party_events.clone().ok_msg("No party events found")?;
    let usd = pev.usd_rdg_estimate().unwrap();
    info!("RDG USD Highest Bid estimate: ${:.2}", usd);
    price_map.insert(SupportedCurrency::Redgold, usd);

    let amount = if s.not_usd {
        CurrencyAmount::from_fractional_cur(s.amount, s.input_currency.clone()).unwrap()
    } else {
        CurrencyAmount::from_fractional_cur(s.amount / usd, s.input_currency.clone()).unwrap()
    };

    // pid

    let ai = c.address_info_for_pk(&hot_pk).await;
    let bal = ai.clone().ok().map(|x| x.balance).unwrap_or(0);
    let cur = SupportedCurrency::Redgold;
    let b = CurrencyAmount::from(bal);
    info!("Balance for {}: {} {} -- ${:.2} USD", cur.to_display_string(), b.to_fractional(), cur.abbreviated(),
            (b.to_fractional() * price_map.get(&cur).unwrap()));

    let ai = ai.ok();

    let channel = Channel::new();

    let jh = create_swap_tx(&g, &res, party_key.clone(), s.input_currency.clone(), hot_pk, hot_kp, amount,
                     &(*nc).clone(), ai, channel.clone(), s.output_currency.clone());

    jh.await.unwrap();

    let update = channel.receiver.recv().unwrap();
    let txid = match update {
        LocalStateUpdate::SwapResult(r) => {
            info!("Swap created");
            let prepared = r.unwrap();
            let signed = prepared.sign(res.clone(), g).await.unwrap();
            let result = signed.broadcast(res).await.unwrap();
            info!("Swap broadcasted: {}", result.broadcast_response);
            if let Some(t) = result.tx.as_ref() {
                info!("Internal TXID {}", t.hash_hex());
                t.hash_hex()
            } else if let Some(txid) = result.txid.as_ref() {
                info!("External TXID {}", txid.identifier.clone());
                txid.identifier.clone()
            } else {
                panic!("No txid found")
            }
        }
        _ => panic!("Unexpected swap result")
    };
    info!("Entered polling loop to check for swap completion");
    let mut attempts = 0;
    loop {

        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        attempts += 1;
        info!("Waited {} minutes", attempts);
        let new_data = c.party_data().await.unwrap();
        let pid = new_data.get(&party_key).cloned().unwrap();
        let pev = pid.party_events.clone().ok_msg("No party events found")?;
        let fulfill = pev.find_fulfillment_of(txid.clone());
        if let Some(f) = fulfill {
            info!("Swap completed");
            info!("Fulfillment identifier: {}", f.2.identifier());
            break;
        }
        if attempts > 10 {
            info!("Swap not completed after 15 minutes");
            break;
        }
    }

    Ok(())
}