use std::fs;
use eframe::egui::debug_text::print;
use redgold_data::data_store::DataStore;
use redgold_rpc_integ::eth::eth_wallet::EthWalletWrapper;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::structs::{Address, CurrencyAmount, Hash, NetworkEnvironment, Transaction, UtxoId};
use redgold_schema::util::lang_util::AnyPrinter;
use redgold_schema::util::times::ToTimeString;
use crate::core::relay::Relay;
use crate::node_config::EnvDefaultNodeConfig;
use crate::party::party_stream::PartyEventBuilder;
use crate::test::external_amm_integration::dev_ci_kp;
use crate::util;

#[ignore]
#[tokio::test]
async fn dbg_main() {
    let n = NodeConfig::by_env_with_args(NetworkEnvironment::Dev).await;
    let r = Relay::new(n.clone()).await;

    let res = r.ds.multiparty_store.all_party_info_with_key().await.unwrap();
    let pi = res.get(0).expect("head");

    let key = pi.party_key.clone().expect("key");
    let data = r.ds.multiparty_store.party_data(&key).await.expect("data")
        .and_then(|pd| pd.json_party_internal_data)
        .and_then(|pid| pid.json_from::<PartyInternalData>().ok()).expect("pid");

    let pev = data.party_events.clone().expect("v");

    let ev = pev.events.clone();

    let dev = dev_ci_kp().expect("works");
    let eth = EthWalletWrapper::new(&dev.0, &NetworkEnvironment::Dev).expect("works");
    let fee = eth.get_fee_estimate().await.unwrap();
    let gas_price = eth.get_gas_price().await.unwrap();
    println!("Expected fee: {}", fee.to_fractional().to_string());
    println!("eth_fee_fixed_normal_by_env {}", CurrencyAmount::eth_fee_fixed_normal_by_env(&NetworkEnvironment::Dev).to_fractional().to_string());
    println!("Gas price hardcoded {}", CurrencyAmount::gas_price_fixed_normal_by_env(&NetworkEnvironment::Dev).to_fractional().to_string());
    println!("Expected gas price: {}", gas_price.to_fractional().to_string());
    // eth.send()

    for o in pev.orders() {
        println!("Order {}", o.event_time.to_time_string_shorter_no_seconds_am_pm());
        println!("Order {}", o.order_amount.to_string());
        println!("Order {}", o.fulfilled_amount.to_string());
        println!("Order {}", o.destination.currency_or().to_display_string());
        println!("----");
    }

}