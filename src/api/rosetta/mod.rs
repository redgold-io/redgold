use futures::TryFuture;
use itertools::Itertools;
use redgold_schema::RgResult;
// use ndarray::s;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::time::Duration;
use warp::Filter;

use redgold_common::client::http::RgHttpClient;
use crate::api::rosetta::models::*;
use crate::api::rosetta::spec::Rosetta;
use crate::core::relay::Relay;
use crate::util::random_port;
use redgold_keys::word_pass_support::WordsPassNodeConfig;
use redgold_keys::TestConstants;
use redgold_schema::conf::node_config::NodeConfig;
// use crate::genesis::create_test_genesis_transaction;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
// use crate::genesis::create_test_genesis_transaction;
use redgold_schema::proto_serde::ProtoHashable;
use redgold_schema::structs::ErrorInfo;
// use crate::genesis::create_test_genesis_transaction;
use redgold_schema::util::lang_util::SameResult;

pub mod models;
pub mod server;
pub mod handlers;
pub mod reject;
pub mod spec;


pub async fn run_test_request<Req, Resp, F>(
    req: Req, f: F, relay: Relay, endpoint: String, server: impl Future<Output=RgResult<()>>)
where
      Req: Serialize + Sized,
      Resp: DeserializeOwned,
      F: FnOnce(Resp) -> () {
    tokio::select! {
        _ = server => {}
        res = test_request::<Req, Resp>(relay.node_config.rosetta_port(), &req, endpoint) => {
            res
            // .map( |r|
            //     log::info!("Test request response: {}", serde_json::to_string(&r.clone()).expect("ser"))
            //     r
            // )
            .map(f).map_err( |e|
                log::error!("Error in test request: {}", serde_json::to_string(&e).expect("ser"))
            ).expect("a")
        }
    }
}


async fn rosetta_relay() -> Relay {
    // util::init_logger();
    let nc = NodeConfig::from_test_id(&(8 as u16));
    let relay = Relay::new(nc).await;
    let mut relay = Relay::default().await;
    let port = random_port();
    relay.node_config.rosetta_port = Some(port);
    relay.clone()
}

pub async fn test_request<Req, Resp>(port: u16, req: &Req, endpoint: String) -> Result<Resp, ErrorInfo>
where
    Req: Serialize + ?Sized,
    Resp: DeserializeOwned
{
    let client = RgHttpClient::new("localhost".to_string(), port, None);
    tokio::time::sleep(Duration::from_secs(2)).await;
    client.json_post::<Req, Resp>(&req, endpoint).await
}

#[ignore]
#[tokio::test]
async fn test() {

    let relay = rosetta_relay().await;
    let tc = TestConstants::new();

    let mut req = AccountBalanceRequest {
        network_identifier: NetworkIdentifier {
            blockchain: "".to_string(),
            network: "".to_string(),
            sub_network_identifier: None
        },
        account_identifier: AccountIdentifier {
            address: "".to_string(),
            sub_account: None,
            metadata: None
        },
        block_identifier: None,
        currencies: None
    };
    use futures::future::FutureExt;
    let srv = server::run_server(relay.clone()).shared();
    run_test_request(req.clone(), |resp: Error| {
        assert_eq!(resp.code, 18)
    }, relay.clone(), "account/balance".to_string(), srv.clone()).await;

    req.network_identifier.network = "debug".to_string();
    run_test_request(req.clone(), |resp: Error| {
        assert_eq!(resp.code, 18)
    }, relay.clone(), "account/balance".to_string(), srv.clone()).await;

    req.network_identifier.blockchain = Rosetta::redgold_blockchain();
    run_test_request(req.clone(), |resp: Error| {
        assert_eq!(resp.code, 14)
    }, relay.clone(), "account/balance".to_string(), srv.clone()).await;

    req.account_identifier.address = tc.address_1.render_string().expect("addr");

    relay.ds.run_migrations().await.expect("migrate");

    // TODO: Replace this
    // relay.ds.transaction_store.insert_transaction(&create_test_genesis_transaction(), 0, true, None, true)
    //     .await.expect("a");
    // relay.ds.insert_block_update_historicals(&create_genesis_block()).await.expect("a");
    // let res = relay.ds.address_block_store.all_address_balance_by_height(0).await.expect("qry");
    // for b in res {
    //     log::info!("Historical balance: {} address: {} ", b.balance, b.address.render_string().expect("r"));
    // }
    //
    // run_test_request(req.clone(), |resp: AccountBalanceResponse| {
    //     assert_eq!(resp.block_identifier.index, 0)
    // }, relay.clone(), "account/balance".to_string(), srv.clone()).await;
    //
    // let ni = NetworkIdentifier{
    //     blockchain: Rosetta::redgold_blockchain(),
    //     network: "debug".to_string(),
    //     sub_network_identifier: None
    // };
    //

}
