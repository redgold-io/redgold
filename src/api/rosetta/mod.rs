use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::sync::Arc;

// use bitcoin_wallet::account::Account;
use futures::future::AndThen;
use futures::TryFuture;
use itertools::Itertools;
use log::info;
// use ndarray::s;
use serde::de::DeserializeOwned;
use serde::Serialize;
use strum_macros::EnumString;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use warp::{Filter, Rejection};
use warp::http::StatusCode;
use warp::path::Exact;
use warp::reply::{Json, WithStatus};

use redgold_schema::{constants, TestConstants};

use crate::{schema, util};
use crate::api::rosetta::models::*;
use crate::api::rosetta::spec::Rosetta;
use crate::core::relay::Relay;
use crate::data::data_store::DataStore;
use crate::genesis::{create_genesis_block, create_genesis_transaction};
use crate::node_config::NodeConfig;
use crate::schema::{bytes_data, error_message};
use crate::schema::{
    from_hex, i64_from_string, ProtoHashable, SafeBytesAccess, WithMetadataHashable,
};
use crate::schema::structs;
use crate::schema::structs::{
    Address, Error as RGError, Input, Output, Proof, State, StructMetadata,
    SubmitTransactionRequest, UtxoEntry,
};
use crate::schema::structs::{ErrorInfo, Hash};
use crate::schema::transaction::amount_data;
use crate::util::lang_util::SameResult;
use crate::util::random_port;

pub mod models;
pub mod server;
pub mod handlers;
pub mod reject;
pub mod spec;


pub async fn run_test_request<Req, Resp, F>(
    req: Req, f: F, relay: Relay, endpoint: String, server: impl Future<Output=()>)
where
      Req: Serialize + Sized,
      Resp: DeserializeOwned,
      F: FnOnce(Resp) -> () {
    tokio::select! {
        _ = server => {}
        res = crate::api::RgHttpClient::test_request::<Req, Resp>(relay.node_config.rosetta_port(), &req, endpoint) => {
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
    let mut relay = Relay::default().await;
    let port = random_port();
    relay.node_config.rosetta_port = Some(port);
    relay.clone()
}

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

    req.network_identifier.network = "debug".into();
    run_test_request(req.clone(), |resp: Error| {
        assert_eq!(resp.code, 18)
    }, relay.clone(), "account/balance".to_string(), srv.clone()).await;

    req.network_identifier.blockchain = Rosetta::redgold_blockchain();
    run_test_request(req.clone(), |resp: Error| {
        assert_eq!(resp.code, 14)
    }, relay.clone(), "account/balance".to_string(), srv.clone()).await;

    req.account_identifier.address = tc.address_1.render_string().expect("addr");

    relay.ds.run_migrations().await.expect("migrate");

    relay.ds.transaction_store.insert_transaction(&create_genesis_transaction(), 0, true, None)
        .await.expect("a");
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
