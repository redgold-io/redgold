use crate::core::relay::Relay;
use crate::party::party_stream::PartyEventBuilder;
use crate::test::external_amm_integration::dev_ci_kp;
use csv::Reader;
use redgold_keys::address_external::ToEthereumAddress;
use redgold_rpc_integ::eth::eth_wallet::EthWalletWrapper;
use redgold_rpc_integ::eth::historical_client::EthHistoricalClient;
use redgold_schema::helpers::easy_json::EasyJsonDeser;
use redgold_schema::party::address_event::AddressEvent;
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::structs::{CurrencyAmount, NetworkEnvironment, SupportedCurrency};
use redgold_schema::util::times::ToTimeString;
use redgold_schema::RgResult;
use std::collections::HashMap;

#[ignore]
#[tokio::test]
async fn debug_event_stream2() {
    debug_events2().await.unwrap();
}
async fn debug_events2() -> RgResult<()> {

    let relay = Relay::env_default(NetworkEnvironment::Dev).await;
    relay.ds.run_migrations().await?;

    let res = relay.ds.multiparty_store.all_party_info_with_key().await?;
    let pi = res.get(0).expect("head");

    let key = pi.party_key.clone().expect("key");
    let data = relay.ds.multiparty_store.party_data(&key).await.expect("data")
        .and_then(|pd| pd.json_party_internal_data)
        .and_then(|pid| pid.json_from::<PartyInternalData>().ok()).expect("pid");

    let pev = data.party_events.clone().expect("v");

    let ev = pev.events.clone();
    let mut bal = CurrencyAmount::zero(SupportedCurrency::Ethereum);

    let (pr, kp) = dev_ci_kp().unwrap();
    let eth = EthWalletWrapper::new(&pr, &NetworkEnvironment::Dev).expect("wallet");
    let actual_bal = eth.get_balance(&key).await.expect("bal");

    for e in ev {
        match e {
            AddressEvent::External(e) => {
                if e.currency == SupportedCurrency::Ethereum {
                    if e.incoming {
                        bal += e.balance_change()
                    } else {
                        bal -= e.balance_change()
                    }
                    println!("hash {}, ts: {}, incoming: {}, amount: {}",
                             e.tx_id, (e.timestamp.unwrap() as i64).to_time_string_shorter_no_seconds_am_pm(),
                             e.incoming,
                             e.balance_change().to_fractional()
                    )
                }
            }
            _ => {}
        }
    }

    println!("Bal: {}", bal.to_fractional());
    println!("Actual Bal: {}", actual_bal.to_fractional());


    let tx = EthHistoricalClient::new(&NetworkEnvironment::Dev).expect("").unwrap()
        .get_all_tx(&key.to_ethereum_address_typed().unwrap().render_string().unwrap(), None
        ).await.expect("txs");
    let mut bal2 = CurrencyAmount::zero(SupportedCurrency::Ethereum);

    for e in tx.iter().rev() {
        println!("hash {}, ts: {}, incoming: {}, amount: {}",
                 e.tx_id, (e.timestamp.unwrap() as i64).to_time_string_shorter_no_seconds_am_pm(),
                 e.incoming,
                 e.balance_change().to_fractional()
        );
        if e.incoming {
            bal2 += e.balance_change()
        } else {
            bal2 -= e.balance_change()
        }
    }

    println!("Bal2: {}", bal2.to_fractional());

    let csv = parse_csv("wtf.csv");
    //
    // let ext = ExternalNetworkResourcesImpl::new(&relay.node_config, Some(relay.clone())).unwrap();
    //
    // let pw = PartyWatcher::new(&relay, ext);
    //
    // for b in pw.get_public_key_btc_data(&key).await.expect("btc").transactions {
    //     println!()
    // }

    Ok(())
}

fn parse_csv(path: &str) -> Vec<HashMap<String, String>> {
    let mut reader = Reader::from_path(path).unwrap();
    let headers = reader.headers().unwrap().clone();

    let records: Vec<HashMap<String, String>> = reader
        .records()
        .filter_map(|record| {
            record.ok().map(|row| {
                headers
                    .iter()
                    .zip(row.iter())
                    .map(|(header, value)| (header.to_string(), value.to_string()))
                    .collect()
            })
        })
        .collect();

    records
}