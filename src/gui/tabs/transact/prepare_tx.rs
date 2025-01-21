use redgold_schema::error_info;
use redgold_schema::structs::{Address, AddressInfo, CurrencyAmount, ErrorInfo, PublicKey, Transaction};
use tracing::info;
use redgold_common_no_wasm::tx_new::TransactionBuilderSupport;
use redgold_keys::address_support::AddressSupport;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::tx::tx_builder::TransactionBuilder;
use redgold_gui::tab::transact::wallet_state::WalletState;
use crate::node_config::{ApiNodeConfig, EnvDefaultNodeConfig};

pub fn prepare_transaction(ai: &AddressInfo, amount: &String, destination: &String, x: &WalletState, nc: &NodeConfig)
                           -> Result<Transaction, ErrorInfo> {
    let destination = destination.parse_address()?;
    let amount = CurrencyAmount::from_float_string(amount)?;
    let mut tb = TransactionBuilder::new(&nc);
    let a = ai.address.as_ref().expect("a");

    tb.with_address_info(ai.clone())?;
    // for u in tb.utxos {
    //     info!("Address info UTXO in prepare transaction: {}", u.json_or());
    // }
    tb.with_output(&destination, &amount);
    // TODO: Fix me
    if x.mark_output_as_swap {
        // tb.with_last_output_swap_type();
    }
    if x.mark_output_as_stake {
        // tb.with_last_output_stake();
        // tb.with_external_stake_usd_bounds(None, None, a, );
    }
    if x.mark_output_as_swap && x.mark_output_as_stake {
        return Err(error_info("Cannot mark as both swap and stake"));
    }
    let res = tb.build();
    res
}

#[ignore]
#[tokio::test]
pub async fn prepare_tx_test() {
    let nc = NodeConfig::dev_default().await;
    let api = nc.api_rg_client();
    let pk_hex = "";
    let pk = PublicKey::from_hex(pk_hex).expect("pk");
    let addr = pk.address().expect("addr");
    let ai = api.address_info(addr.clone()).await.expect("ai");
    let mut tb = TransactionBuilder::new(&nc);
    let a = ai.address.as_ref().expect("a");
    tb.with_address_info(ai.clone()).expect("with_address_info");
    for u in tb.utxos {
        info!("Address info UTXO in prepare transaction: {}", u.json_or());
    }
}
