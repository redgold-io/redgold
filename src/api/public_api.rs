use bytes::Bytes;
use futures::TryFutureExt;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use itertools::Itertools;
use metrics::counter;
use rand::rngs::OsRng;
use rand::RngCore;
use redgold_common::flume_send_help::{new_channel, RecvAsyncErrorInfo, SendErrorInfo};
use redgold_schema::structs::{AboutNodeRequest, AboutNodeResponse, AddressInfo, FaucetRequest, FaucetResponse, HashSearchRequest, HashSearchResponse, NetworkEnvironment, Seed, SupportedCurrency};
use redgold_schema::message::{Response as RResponse};
use redgold_schema::message::Request;
use redgold_schema::transaction::rounded_balance_i64;
use redgold_schema::{empty_public_request, empty_public_response, from_hex, structs, RgResult, SafeOption};
use reqwest::ClientBuilder;
use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::trace;
use tracing::{debug, error, info};
use warp::http::Response;
use warp::reply::Json;
use warp::{Filter, Server};

use crate::api::client::rest;
use crate::api::explorer::server::{extract_ip, process_origin};
use crate::api::faucet::faucet_request;
use crate::api::hash_query::hash_query;
use crate::api::v1::v1_api_routes;
use crate::api::warp_helpers::as_warp_json_response;
use crate::api::{about, explorer};
use crate::core::internal_message::{PeerMessage, TransactionMessage};
use crate::core::relay::Relay;
use crate::core::transport::peer_rx_event_handler::PeerRxEventHandler;
use crate::node_config::ApiNodeConfig;
use crate::schema::response_metadata;
// use crate::genesis::create_test_genesis_transaction;
use crate::schema::structs::{
    Address, AddressType, ErrorInfo, QueryAddressesRequest, QueryTransactionResponse,
};
use crate::schema::structs::{
    PublicRequest, PublicResponse, ResponseMetadata, SubmitTransactionRequest,
    SubmitTransactionResponse,
};
use crate::schema::structs::{QueryAddressesResponse, Transaction};
use crate::schema::{bytes_data, error_info};
use crate::util::runtimes::build_runtime;
use crate::{api, schema, util};
use redgold_data::data_store::DataStore;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::helpers::easy_json::json;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::proto_serde::{ProtoHashable, ProtoSerde};
use redgold_schema::util::lang_util::{AnyPrinter, SameResult};

async fn process_request(request: PublicRequest, relay: Relay) -> Json {
    let response = process_request_inner(request, relay).await.map_err(|e| {
        let mut response1 = empty_public_response();
        response1.response_metadata = Some(e.response_metadata());
        response1
    }).combine();
    warp::reply::json(&response)
}

async fn process_request_inner(request: PublicRequest, relay: Relay) -> Result<PublicResponse, ErrorInfo> {
    // info!(
    //     "Received publicRequest: {}",
    //     serde_json::to_string(&request.clone()).expect("json decode")
    // );
    //
    let mut response1 = empty_public_response();
    //
    // if let Some(submit_request) = request.submit_transaction_request {
    //     // info!(
    //     //     "Received submit transaction request: {}",
    //     //     serde_json::to_string(&submit_request.clone()).unwrap()
    //     // );
    //     // TODO: Replace this with an async channel of some kind.
    //     // later replace it with something as a strict dependency here.
    //     let (sender, receiver) = flume::unbounded::<PublicResponse>();
    //     let message = TransactionMessage {
    //         transaction: submit_request.transaction.as_ref().unwrap().clone(),
    //         response_channel: match submit_request.sync_query_response {
    //             true => Some(sender),
    //             false => None,
    //         },
    //     };
    //     relay
    //         .clone()
    //         .transaction
    //         .sender
    //         .send(message)
    //         .expect("send");
    //
    //     if !submit_request.sync_query_response {
    //         response1.submit_transaction_response = Some(SubmitTransactionResponse {
    //             transaction_hash: submit_request
    //                 .transaction
    //                 .as_ref()
    //                 .expect("tx")
    //                 .hash()
    //                 .into(),
    //             query_transaction_response: None,
    //             transaction: None,
    //         });
    //     } else {
    //         // info!("API server awaiting transaction results");
    //         let recv = receiver.recv_async_err().await;
    //         // (Duration::from_secs(70));
    //         // info!(
    //         // "API server got transaction results or timeout, success: {:?}",
    //         // recv.clone().is_ok()
    //     // );
    //         match recv {
    //             Ok(r) => {
    //                 response1 = r
    //             }
    //             Err(e) => {
    //                 response1.response_metadata = Some(ResponseMetadata {
    //                     success: false,
    //                     error_info: Some(e),
    //                 });
    //             }
    //         }
    //     }
    // }
    match request.query_transaction_request {
        None => {}
        Some(_) => {}
    }
    if let Some(r) = request.query_addresses_request {
        // TODO: make this thing async and map errors to rejections
        let k = relay.ds.transaction_store.utxo_for_addresses(&r.addresses).await?;
        response1.query_addresses_response = Some(QueryAddressesResponse { utxo_entries: k });
    }
    if let Some(r) = request.about_node_request {
        response1.about_node_response = Some(about::handle_about_node(r, relay.clone()).await?);
    }
    if let Some(r) = request.hash_search_request {
        let res = hash_query(relay.clone(), r.search_string, None, None).await?;
        response1.hash_search_response = Some(res);
    }

    // if let Some(f) = request.faucet_request {
    //     if let Some(a) = f.address {
    //         let fr = faucet_request(a.render_string()?, relay.clone()).await?;
    //         response1.faucet_response = Some(fr);
    //     }
    // }
    Ok(response1)

    // info!("Public API response: {}", serde_json::to_string(&response1.clone()).unwrap());
}

pub async fn as_warp_proto_bytes<T: ProtoSerde>(response: Result<T, ErrorInfo>) -> Response<Vec<u8>> {
    let vec = response
        .map(|r| r.proto_serialize())
        .map_err(|e| RResponse::from_error_info(e).proto_serialize())
        .combine();
    Response::builder().body(vec).expect("a")
}

pub async fn handle_proto_post(reqb: Bytes, _address: Option<SocketAddr>, relay: Relay, origin: Option<String>) -> Result<RResponse, ErrorInfo> {
    counter!("redgold.api.handle_proto_post").increment(1);
    let vec_b = reqb.to_vec();
    let mut request = Request::proto_deserialize(vec_b)?;
    request.origin = origin;
    relay.receive_request_send_internal(request, None).await
}

pub async fn run_server(relay: Relay) -> Result<(), ErrorInfo>{
    let relay2 = relay.clone();

    let hello = warp::get()
        .and(warp::path("hello")).and_then(|| async move {
        let res: Result<&str, warp::reject::Rejection> = Ok("hello");
        res
    });

    let home = warp::get()
        .and_then(|| async move {
        let res: Result<&str, warp::reject::Rejection> = Ok("hello");
        res
    });

    // This is the type PublicRequest
    let trelay = relay.clone();
    let transaction = warp::post()
        .and(warp::path("request"))
        // Only accept bodies smaller than 16kb...
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::json::<PublicRequest>())
        .and_then(move |request: PublicRequest| {
            let relay3 = trelay.clone();
            async move {
                let res: Result<Json, warp::reject::Rejection> =
                    Ok(process_request(request, relay3.clone()).await);
                res
            }
        });

    let qry_relay = relay.clone();

    let query_hash = warp::get()
        .and(warp::path("query"))
        .and(warp::path::param())
        .and_then(move |address: String| {
            let relay3 = qry_relay.clone();
            async move {
                let res: Result<Json, warp::reject::Rejection> =
                    Ok(hash_query(relay3.clone(), address, None, None).await
                        .map_err(|e| warp::reply::json(&e))
                        .map(|r| warp::reply::json(&r))
                        .combine()
                    );
                res
            }
        });

    let a_relay = relay.clone();

    let about = warp::get()
        .and(warp::path("about"))
        .and_then(move || {
            let relay3 = a_relay.clone();
            async move {
                // TODO call about handler
                // TODO: Should this be hitting the peer message channel?
                let abr = about::handle_about_node(AboutNodeRequest::default(), relay3.clone()).await;
                as_warp_json_response(abr)
            }
        });

    let p_relay = relay.clone();
    let peers = warp::get()
        .and(warp::path("peers"))
        .and_then(move || {
            let relay3 = p_relay.clone();
            async move {
                // TODO: Make this a standard request way easier
                let ps = relay3.ds.peer_store.all_peers_tx().await;
                let res: Result<Json, warp::reject::Rejection> = as_warp_json_response(ps);
                res
            }
        });

    let address_relay = relay.clone();
    let address_lookup = warp::get()
        .and(warp::path("address"))
        .and(warp::path::param())
        .and_then(move |hash: String| {
            let relay3 = address_relay.clone();
            async move {
                let ps = relay3.ds.get_address_string_info(hash).await;
                let res: Result<Json, warp::reject::Rejection> = Ok(ps
                       .map_err(|e| warp::reply::json(&e))
                       .map(|r| warp::reply::json(&r))
                       .combine());
                res
            }
        });

    let tmp_relay = relay.clone();
    let public = warp::get()
        .and(warp::path("public"))
        .and_then(move || {
            let relay3 = tmp_relay.clone();
            async move {
                let ps: RgResult<structs::PublicKey> = Ok(relay3.node_config.public_key());
                as_warp_json_response(ps)
            }
        });

    let tmp_relay = relay.clone();
    let peer_id = warp::get()
        .and(warp::path("peer-id"))
        .and_then(move || {
            let relay3 = tmp_relay.clone();
            async move {
                let ps = relay3.peer_id_from_node_tx().await;
                as_warp_json_response(ps)
            }
        });

    let tmp_relay = relay.clone();
    let node_tx = warp::get()
        .and(warp::path("node-tx"))
        .and_then(move || {
            let relay3 = tmp_relay.clone();
            async move {
                let ps = relay3.node_tx().await;
                as_warp_json_response(ps)
            }
        });

    let tmp_relay = relay.clone();
    let peer_tx = warp::get()
        .and(warp::path("peer-tx"))
        .and_then(move || {
            let relay3 = tmp_relay.clone();
            async move {
                let ps = relay3.peer_tx().await;
                as_warp_json_response(ps)
            }
        });

    let tmp_relay = relay.clone();
    let trust = warp::get()
        .and(warp::path("trust"))
        .and_then(move || {
            let relay3 = tmp_relay.clone();
            async move {
                let ps = relay3.get_trust().await;
                as_warp_json_response(ps)
            }
        });

    let bin_relay = relay.clone();

    let request_normal = warp::post()
        .and(warp::path("request_peer"))
        .and(warp::body::json::<Request>())
        .and(warp::addr::remote())
        .and_then(move |request: Request, _address: Option<SocketAddr>| {
            let relay3 = bin_relay.clone();
            async move {
                // TODO: Isn't this supposed to go to peerRX event handler?
                // info!{"Warp request from {:?}", address};
                let res: Result<Json, warp::reject::Rejection> = {
                    let response = relay3.receive_request_send_internal(request, None).await;
                    //PeerRxEventHandler::request_response(relay3, request, )
                    Ok(response
                        .map_err(|e| warp::reply::json(&e))
                        .map(|r| warp::reply::json(&r))
                        .combine()
                    )
                };
                res
            }
        });

    let bin_relay2 = relay.clone();

    let request_bin = warp::post()
        .and(warp::path("request_proto"))
        .and(warp::body::bytes())
        .and(warp::addr::remote())
        .and(extract_ip())
        .and_then(move |reqb: Bytes, address: Option<SocketAddr>, remote: Option<String>| {
            // TODO: verify auth and receive message sync from above
            let relay3 = bin_relay2.clone();
            let origin = process_origin(address, remote, relay3
                .node_config.config_data.node.as_ref().and_then(|c| c.allowed_http_proxy_origins.clone())
                .unwrap_or(vec![])
            );
            let result = async move {
                let res: Result<Response<Vec<u8>>, warp::Rejection> =
                    Ok(as_warp_proto_bytes(handle_proto_post(reqb, address, relay3.clone(), origin).await).await);
                res
            };
            result
        });

    let port = relay2.node_config.public_port();

    let relay_arc = Arc::new(relay2.clone());

    let routes = hello
        .or(trust)
        .or(peer_tx)
        .or(node_tx)
        .or(peer_id)
        .or(public)
        .or(transaction)
        // .or(faucet)
        .or(query_hash)
        .or(about)
        .or(request_normal)
        .or(request_bin)
        .or(peers)
        .or(address_lookup)
        // .or(explorer_hash)
        // .or(explorer_recent)
        .or(explorer::server::explorer_specific_routes(relay2.clone()))
        .or(v1_api_routes(relay_arc))
        .or(home);

    // Create a warp Service using the filter
    // Create the server
    let addr = SocketAddr::from(([0, 0, 0, 0], port));


    //
    // let folder = relay2.node_config.data_folder.all();
    //
    // let cert = if let (Ok(cert), Ok(key)) = (folder.cert().await, folder.key().await) {
    //     Some((cert, key))
    // } else {
    //     info!("Unable to find TLS / SSL cert in: {}", folder.path.to_str().unwrap().to_string());
    //     None
    // };
    //

    let server =
    //     if let Some((cert, key)) = cert {
    //     info!("Using SSL/TLS on public API");
    //     warp::serve(routes)
    //         .tls()
    //         .cert(cert)
    //         .key(key)
    //         .run(addr)
    //         .await
    // } else {
        warp::serve(routes)
            .run(addr)
            .await;
    // };
     relay2.node_config.network.to_std_string().print();

    Ok(server)
}

#[derive(serde::Deserialize, Default, Clone)]
pub struct Pagination {
    pub offset: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(serde::Deserialize)]
pub struct TokenParam {
    pub token: Option<String>,
}

pub fn start_server(relay: Relay
                    // , runtime: Arc<Runtime>
) -> JoinHandle<Result<(), ErrorInfo>> {

    let handle = tokio::spawn(run_server(relay.clone()));
    trace!("Started PublicAPI server on port {:?}", relay.clone().node_config.public_port());
    return handle;
}
//
// #[allow(dead_code)]
// async fn mock_relay(relay: Relay) {
//     loop {
//         let tm = relay.mempool.receiver.recv().unwrap();
//         let mut response = structs::Response::default();
//         response.submit_transaction_response = Some(SubmitTransactionResponse {
//                 transaction_hash: create_test_genesis_transaction().hash_or().into(),
//                 query_transaction_response: Some(QueryTransactionResponse {
//                     observation_proofs: vec![]
//                 }),
//             transaction: None,
//         });
//         tm.response_channel
//             .unwrap()
//             .send(response)
//             .expect("send");
//     }
// }

#[ignore]
#[test]
fn test_public_api_lb() {
    let mut config = NodeConfig::default();
    config.network = NetworkEnvironment::Dev;
    let c = config.api_client();
    let rt = build_runtime(1, "test");
    println!("{:?}", rt.block_on(c.about()));
}