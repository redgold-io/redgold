use crate::gui::app_loop::LocalState;
use crate::node_config::ApiNodeConfig;
use flume::Sender;
use redgold_common::flume_send_help::SendErrorInfo;
use redgold_gui::dependencies::gui_depends::GuiDepends;
use redgold_gui::state::local_state::{BalanceAddressInfoUpdate, LocalStateUpdate};
use redgold_keys::address_external::ToEthereumAddress;
use redgold_keys::btc::btc_wallet::SingleKeyBitcoinWallet;
use redgold_rpc_integ::eth::historical_client::EthHistoricalClient;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::structs::{PublicKey, SupportedCurrency};
use redgold_schema::transaction::rounded_balance_i64;
use std::collections::HashMap;
use tracing::{error, info};

pub fn get_address_info<G>(
    node_config: &NodeConfig,
    public_key: PublicKey,
    update_channel: Sender<LocalStateUpdate>,
    g: &G,
)  where G: GuiDepends + Clone + Send + 'static  {
    let node_config = node_config.clone();
    let address = public_key.address().expect("works");
    let _ = g.spawn(async move {
        let environment = node_config.network.clone();
        // info!("Getting balance for environment: {}", environment.to_std_string());
        let mut balances: HashMap<SupportedCurrency, f64> = Default::default();
        let btc_bal = SingleKeyBitcoinWallet::new_wallet(
            public_key.clone(), environment, true)
                .ok().and_then(|w| w.get_wallet_balance().ok())
                .map(|b| b.confirmed as f64 / 1e8f64);
        balances.insert(SupportedCurrency::Bitcoin, btc_bal.unwrap_or(0.0));
        let mut eth_bal: Option<f64> = None;
        if let Some(Ok(eth)) = EthHistoricalClient::new(&environment) {
            let eth_addr = public_key.to_ethereum_address().expect("eth");
            if let Ok(bi) = eth.get_balance(&eth_addr).await {
                if let Ok(v) = EthHistoricalClient::translate_value_to_float(&bi) {
                    eth_bal = Some(v);
                }
            }
        }
        balances.insert(SupportedCurrency::Ethereum, eth_bal.unwrap_or(0.0));

        let client = node_config.api_rg_client();
        let response = client
            .address_info_for_pk(&public_key).await;
        // let fun: Box<dyn FnMut(&mut LocalState) + Send> =
        let mut address_info = None;
        match response {
            Ok(ai) => {
                address_info = Some(ai.clone());
                let o = rounded_balance_i64(ai.balance.clone());
                balances.insert(SupportedCurrency::Redgold, o.clone());

                // info!("balance success: {}", ai.json_or());
                // Box::new(move |ls: &mut LocalState| {
                    // info!("Applied update function inside closure for balance thing");
                    // ls.wallet.address_info = Some(ai.clone());
                    // ls.wallet.balance = o.to_string();
                    // ls.wallet.balance_f64 = Some(o.clone());
                    // ls.wallet.balance_btc_f64 = btc_bal.clone();
                    // ls.wallet.balance_btc = btc_bal.clone().map(|b| b.to_string());
                    // ls.wallet.balance_eth_f64 = eth_bal.clone();
                    // ls.wallet.balance_eth = eth_bal.clone().map(|b| b.to_string());
                    //
                // })
            }
            Err(e) => {
                error!("balance error: {}", e.json_or());
                // Box::new(move |ls: &mut LocalState| {
                //     ls.wallet.balance = "error".to_string();
                // })
            }
        };

        let up = LocalStateUpdate::BalanceUpdates(BalanceAddressInfoUpdate{
            balances: balances.clone(),
            address_info: address_info.clone(),
        });
        // let up = StateUpdate {
        //     update: fun,
        // };
        update_channel.send_rg_err(up).log_error().ok();
    });
}
