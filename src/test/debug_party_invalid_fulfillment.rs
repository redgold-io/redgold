use itertools::Itertools;
use log::info;
use redgold_data::data_store::DataStore;
use redgold_keys::address_external::ToEthereumAddress;
use redgold_keys::eth::eth_wallet::EthWalletWrapper;
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::{RgResult, structs};
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{NetworkEnvironment, SupportedCurrency};
use redgold_schema::util::lang_util::AnyPrinter;
use crate::api::explorer::convert_events;
use crate::core::relay::Relay;
use crate::core::transact::tx_builder_supports::{TransactionBuilder, TransactionBuilderSupport};
use crate::party::data_enrichment::PartyInternalData;
use crate::party::party_stream::PartyEvents;

#[ignore]
#[tokio::test]
async fn debug_event_stream2() {
    debug_events2().await.unwrap();
}
async fn debug_events2() -> RgResult<()> {
    let home = dirs::home_dir().expect("home");
    let hnds_path = home.join(".rg/hostnoc/main/data_store.sqlite".to_string());
    let dshn = DataStore::from_config_path(&hnds_path).await;

    let hn_info = dshn.multiparty_store.all_party_info_with_key().await?.get(0).expect("head").clone()
        .party_key.clone().expect("key");;
    let hn_pid = dshn.multiparty_store.party_data(&hn_info).await.expect("data")
        .and_then(|pd| pd.json_party_internal_data)
        .and_then(|pid| pid.json_from::<PartyInternalData>().ok()).expect("pid");
    let hn_pev = hn_pid.party_events.clone().expect("v");


    hn_pev.json_or().print();

    let relay = Relay::env_default(NetworkEnvironment::Main).await;
    relay.ds.run_migrations().await?;

    let res = relay.ds.multiparty_store.all_party_info_with_key().await?;
    let pi = res.get(0).expect("head");

    let key = pi.party_key.clone().expect("key");
    let data = relay.ds.multiparty_store.party_data(&key).await.expect("data")
        .and_then(|pd| pd.json_party_internal_data)
        .and_then(|pid| pid.json_from::<PartyInternalData>().ok()).expect("pid");

    // info!("Party key {}", key.hex());

    let mut pev = data.party_events.clone().expect("v");
    pev.json_or().print();

    let ev = pev.events.clone();

    // let mut duplicate = PartyEvents::new(&key, &NetworkEnvironment::Dev, &relay);
    // convert_events(data.clone(), &relay.node_config).expect("convert").json_pretty_or().print();
    //
    // // this matches
    // for e in &ev {
    //     duplicate.process_event(e).await.expect("works");
    // }

    let order = hn_pev.orders().get(0).expect("order").clone();
    // order.json_pretty_or().print();
    let eth = relay.eth_wallet()?;
    let mp_eth_addr = key.to_ethereum_address_typed()?;
    let dest = order.destination.clone();
    let fulfilled_currency = order.fulfilled_currency_amount();
    let mut tb = TransactionBuilder::new(&relay.node_config);
    tb.with_input_address(&key.address().expect("works"));
    tb.with_auto_utxos().await?;

    let orig_orders = hn_pev.orders();
    let orders = orig_orders.iter()
        // .filter(|e| e.event_time < cutoff_time)
        .filter(|e| e.destination.currency_or() == SupportedCurrency::Redgold)
        .collect_vec();
    for o in orders.clone() {
        tb.with_output(&o.destination, &o.fulfilled_currency_amount());
        if let Some(a) = o.stake_withdrawal_fulfilment_utxo_id.as_ref() {
            tb.with_last_output_stake_withdrawal_fulfillment(a).expect("works");
        } else {
            tb.with_last_output_deposit_swap_fulfillment(o.tx_id_ref.clone().expect("Missing tx_id")).expect("works");
        };
    }

    let tx = tb.build().expect("build");
    pev.relay = Some(relay.clone());
    pev.validate_rdg_swap_fulfillment_transaction(&tx).expect("");

    //
    // let tx = eth.create_transaction_typed(
    //     &mp_eth_addr, &dest, fulfilled_currency, None
    // ).await?;
    // let data = EthWalletWrapper::signing_data(&tx)?;
    // let tx_ser = tx.json_or();
    // let mut valid = structs::PartySigningValidation::default();
    // valid.json_payload = Some(tx_ser.clone());
    // valid.currency = SupportedCurrency::Ethereum as i32;
    //
    // pev.validate_eth_fulfillment(tx_ser, data)?;
    // // let past_orders = pev.fulfillment_history.iter().map(|x| x.0.clone()).collect_vec();
    //


    //
    // let mut tb = relay.node_config.tx_builder();
    // tb.with_input_address(&key.address().expect("works"))
    //     .with_auto_utxos().await.expect("works");
    //
    // for o in past_orders.iter()
    //     // .filter(|e| e.event_time < cutoff_time)
    //     .filter(|e| e.destination.currency_or() == SupportedCurrency::Redgold) {
    //     tb.with_output(&o.destination, &o.fulfilled_currency_amount());
    //     if let Some(a) = o.stake_withdrawal_fulfilment_utxo_id.as_ref() {
    //         tb.with_last_output_stake_withdrawal_fulfillment(a).expect("works");
    //     } else {
    //         tb.with_last_output_deposit_swap_fulfillment(o.tx_id_ref.clone().expect("Missing tx_id")).expect("works");
    //     };
    // }
    //
    // if tb.transaction.outputs.len() > 0 {
    //     let tx = tb.build()?;
    //     // pev.validate_rdg_swap_fulfillment_transaction(&tx)?;
    //     // info!("Sending RDG fulfillment transaction: {}", tx.json_or());
    //     // self.mp_send_rdg_tx(&mut tx.clone(), identifier.clone()).await.log_error().ok();
    //     // info!("Sent RDG fulfillment transaction: {}", tx.json_or());
    // }
    // tb.transaction.json_pretty_or().print();
    // Ok(())

    // pev.json_pretty_or().print();
    // not this
    //
    // let cent = pev.central_prices.get(&SupportedCurrency::Bitcoin).expect("redgold");
    //
    //     cent.json_pretty_or().print();
    // cent.fulfill_taker_order(10_000, true, 1722524343044, None, &Address::default()).json_pretty_or().print();
    Ok(())
    // let pk_hex = "024cfc97a479af32fcb9d7b59c0e1273832817bf0bb264227e56e449d1a6b30e8e";
    // let pk_address = PublicKey::from_hex_direct(pk_hex).expect("pk");
    //
    // let eth_addr = "0x7D464545F9E9E667bbb1A907121bccb49Dc39160".to_string();
    // let eth = EthHistoricalClient::new(&NetworkEnvironment::Dev).expect("").expect("");
    // let tx = eth.get_all_tx(&eth_addr, None).await.expect("");
    //
    // let mut events = vec![];
    // for e in &tx {
    //     events.push(External(e.clone()));
    // };
    //
    // let mut pq = PriceDataPointUsdQuery::new();
    // pq.enrich_address_events(&mut events, &relay.ds).await.expect("works");
    //
    // let mut pe = PartyEvents::new(&pk_address, &NetworkEnvironment::Dev, &relay);
    //
    //
    // for e in &events {
    //
    //     pe.process_event(e).await?;
    // }
    //
    //
    // println!("{}", pe.json_or());
    //
    // Ok(())

}