use redgold_schema::{EasyJson, error_info};
use redgold_schema::structs::{Address, AddressInfo, CurrencyAmount, ErrorInfo, PublicKey, Transaction};
use tracing::info;
use crate::core::transact::tx_builder_supports::{TransactionBuilder, TransactionBuilderSupport};
use crate::gui::tabs::transact::wallet_tab::WalletState;
use crate::node_config::NodeConfig;

pub fn prepare_transaction(ai: &AddressInfo, amount: &String, destination: &String, x: &WalletState, nc: &NodeConfig)
                           -> Result<Transaction, ErrorInfo> {
    let destination = Address::parse(destination.clone())?;
    let amount = CurrencyAmount::from_float_string(amount)?;
    let mut tb = TransactionBuilder::new(&nc);
    let a = ai.address.as_ref().expect("a");

    tb.with_address_info(ai.clone())?;
    // for u in tb.utxos {
    //     info!("Address info UTXO in prepare transaction: {}", u.json_or());
    // }
    tb.with_output(&destination, &amount);
    if x.mark_output_as_swap {
        tb.with_last_output_withdrawal_swap();
    }
    if x.mark_output_as_stake {
        tb.with_last_output_stake();
        tb.with_stake_usd_bounds(None, None, a);
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
    let api = nc.api_client();
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
