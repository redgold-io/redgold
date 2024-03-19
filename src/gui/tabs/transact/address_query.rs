use redgold_keys::eth::example::EthHistoricalClient;
use redgold_keys::util::btc_wallet::SingleKeyBitcoinWallet;
use redgold_schema::structs::PublicKey;
use tracing::{error, info};
use redgold_keys::address_external::ToEthereumAddress;
use redgold_schema::EasyJson;
use redgold_schema::transaction::rounded_balance_i64;
use crate::core::internal_message::SendErrorInfo;
use crate::gui::app_loop::LocalState;
use crate::gui::tabs::transact::wallet_tab::StateUpdate;
use crate::node_config::NodeConfig;
use crate::observability::logging::Loggable;

pub fn get_address_info(
    node_config: &NodeConfig,
    public_key: PublicKey,
    update_channel: flume::Sender<StateUpdate>,
) {
    let node_config = node_config.clone();
    let address = public_key.address().expect("works");
    let _ = tokio::spawn(async move {

        let btc_bal = SingleKeyBitcoinWallet::new_wallet(
                public_key.clone(), node_config.network.clone(), true)
                .ok().and_then(|w| w.get_wallet_balance().ok())
                .map(|b| b.confirmed as f64 / 1e8f64);

        let mut eth_bal: Option<f64> = None;
        if let Some(Ok(eth)) = EthHistoricalClient::new(&node_config.network) {
            let eth_addr = public_key.to_ethereum_address().expect("eth");
            if let Ok(bi) = eth.get_balance(&eth_addr).await {
                if let Ok(v) = EthHistoricalClient::translate_value_to_float(&bi) {
                    eth_bal = Some(v);
                }
            }
        }

        let client = node_config.api_client();
        let response = client
            .address_info(address).await;
        let fun: Box<dyn FnMut(&mut LocalState) + Send> = match response {
            Ok(ai) => {
                info!("balance success: {}", ai.json_or());
                Box::new(move |ls: &mut LocalState| {
                    info!("Applied update function inside closure for balance thing");
                    let o = rounded_balance_i64(ai.balance.clone());
                    ls.wallet_state.balance = o.to_string();
                    ls.wallet_state.balance_f64 = Some(o.clone());
                    ls.wallet_state.address_info = Some(ai.clone());
                    ls.wallet_state.balance_btc_f64 = btc_bal.clone();
                    ls.wallet_state.balance_btc = btc_bal.clone().map(|b| b.to_string());
                    ls.wallet_state.balance_eth_f64 = eth_bal.clone();
                    ls.wallet_state.balance_eth = eth_bal.clone().map(|b| b.to_string());
                })
            }
            Err(e) => {
                error!("balance error: {}", e.json_or());
                Box::new(move |ls: &mut LocalState| {
                    ls.wallet_state.balance = "error".to_string();
                })
            }
        };
        let up = StateUpdate {
            update: fun,
        };
        update_channel.send_err(up).log_error().ok();
    });
}
