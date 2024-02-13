use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use bitcoin::util::misc::hex_bytes;
use bytes::Bytes;
use futures::TryFutureExt;

use itertools::Itertools;
use log::{debug, error, info};
use rand::rngs::OsRng;
use rand::RngCore;
use reqwest::ClientBuilder;
use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use warp::reply::Json;
use warp::{Filter, Server};
use warp::http::Response;
use redgold_schema::{empty_public_request, empty_public_response, from_hex, json, ProtoHashable, ProtoSerde, RgResult, SafeOption, structs};
use redgold_schema::structs::{AboutNodeRequest, AboutNodeResponse, AddressInfo, FaucetRequest, FaucetResponse, HashSearchRequest, HashSearchResponse, NetworkEnvironment, Request, Response as RResponse, Seed};
use redgold_schema::transaction::rounded_balance_i64;

use crate::core::internal_message::{new_channel, PeerMessage, RecvAsyncErrorInfo, SendErrorInfo, TransactionMessage};
use crate::core::relay::Relay;
use redgold_data::data_store::DataStore;
use crate::genesis::create_test_genesis_transaction;
use crate::schema::structs::{
    Address, AddressType, ErrorInfo, QueryAddressesRequest, QueryTransactionResponse,
};
use crate::schema::structs::{
    PublicRequest, PublicResponse, ResponseMetadata, SubmitTransactionRequest,
    SubmitTransactionResponse,
};
use crate::schema::structs::{QueryAddressesResponse, Transaction};
use crate::schema::{bytes_data, error_info};
use crate::schema::{response_metadata, SafeBytesAccess, WithMetadataHashable};
use crate::{api, schema, util};
use crate::api::{about, as_warp_json_response, explorer};
use crate::api::faucet::faucet_request;
use crate::api::hash_query::hash_query;
use crate::core::peer_rx_event_handler::PeerRxEventHandler;
use crate::node_config::NodeConfig;
use redgold_schema::util::lang_util::SameResult;
use crate::util::runtimes::build_runtime;

// https://github.com/rustls/hyper-rustls/blob/master/examples/server.rs

#[derive(Clone)]
pub struct PublicClient {
    pub url: String,
    pub port: u16,
    pub timeout: Duration,
    pub relay: Option<Relay>
}

impl PublicClient {
    // pub fn default() -> Self {
    //     PublicClient::local(3030)
    // }

    pub fn client_wrapper(&self) -> api::RgHttpClient {
        api::RgHttpClient::new(self.url.clone(), self.port as u16, self.relay.clone())
    }

    pub fn local(port: u16, _relay: Option<Relay>) -> Self {
        Self {
            url: "localhost".to_string(),
            port,
            timeout: Duration::from_secs(30),
            relay: None,
        }
    }

    pub fn from(url: String, port: u16, relay: Option<Relay>) -> Self {
        Self {
            url,
            port,
            timeout: Duration::from_secs(30),
            relay,
        }
    }


    fn formatted_url(&self) -> String {
        return "http://".to_owned() + &*self.url.clone() + ":" + &*self.port.to_string();
    }

    fn formatted_url_metrics(&self) -> String {
        return "http://".to_owned() + &*self.url.clone() + ":" + &*(self.port - 2).to_string();
    }

    #[allow(dead_code)]
    pub async fn request(&self, r: &PublicRequest) -> Result<PublicResponse, ErrorInfo> {
        use reqwest::ClientBuilder;
        let client = ClientBuilder::new().timeout(self.timeout).build().unwrap();
        // // .default_headers(headers)
        // // .gzip(true)
        // .timeout(self.timeout)
        // .build()?;
        // info!("{:?}", "");
        // info!(
        //     "Sending PublicRequest: {:?}",
        //     serde_json::to_string(&r.clone()).unwrap()
        // );
        let sent = client
            .post(self.formatted_url() + "/request")
            .json(r)
            .send();
        let response = sent.await;
        match response {
            Ok(r) => match r.json::<PublicResponse>().await {
                Ok(res) => Ok(res),
                Err(e) => Err(schema::error_info(e.to_string())),
            },
            Err(e) => Err(error_info(e.to_string())),
        }
    }

    pub async fn metrics(&self) -> Result<String, ErrorInfo>  {
        let client = ClientBuilder::new().timeout(self.timeout).build().unwrap();
        let sent = client
            .get(self.formatted_url_metrics() + "/metrics")
            .send();
        let response = sent.await.map_err(|e | error_info(e.to_string()))?;
        let x = response.text().await;
        let text = x.map_err(|e | error_info(e.to_string()))?;
        Ok(text)
    }

    pub async fn send_transaction(
        &self,
        t: &Transaction,
        sync: bool,
    ) -> Result<SubmitTransactionResponse, ErrorInfo> {

        let c = self.client_wrapper();

        let mut request = Request::default();
        request.submit_transaction_request = Some(SubmitTransactionRequest {
                transaction: Some(t.clone()),
                sync_query_response: sync,
        });
        // debug!("Sending transaction: {}", t.clone().hash_hex_or_missing());
        let response = c.proto_post_request(&mut request, None).await?;
        response.as_error_info()?;
        Ok(response.submit_transaction_response.safe_get()?.clone())
    }


    pub async fn faucet(
        &self,
        t: &Address
    ) -> Result<FaucetResponse, ErrorInfo> {
        let mut request = empty_public_request();
        request.faucet_request = Some(FaucetRequest {
            address: Some(t.clone()),
        });
        info!("Sending faucet request: {}", t.clone().render_string().expect("r"));
        let response = self.request(&request).await?.as_error()?;
        let res = json(&response)?;
        info!("Faucet response: {}", res);
        Ok(response.faucet_response.safe_get_msg(res)?.clone())
    }

    #[allow(dead_code)]
    pub async fn query_addresses(
        &self,
        addresses: Vec<Vec<u8>>,
    ) -> Result<PublicResponse, ErrorInfo> {
        let mut request = empty_public_request();
        request.query_addresses_request = Some(QueryAddressesRequest {
                addresses: addresses
                    .iter()
                    .map(|a| Address {
                        address: bytes_data(a.clone()),
                        address_type: AddressType::Sha3224ChecksumPublic as i32,
                        currency: None,
                    })
                    .collect_vec(),
            });
        self.request(&request).await
    }

    #[allow(dead_code)]
    pub async fn query_address(
        &self,
        addresses: Vec<Address>,
    ) -> Result<PublicResponse, ErrorInfo> {
        let mut request = empty_public_request();
        request.query_addresses_request = Some(QueryAddressesRequest {
                addresses
            });
        self.request(&request).await
    }

    #[allow(dead_code)]
    pub async fn query_hash(
        &self,
        input: String,
    ) -> Result<HashSearchResponse, ErrorInfo> {
        let mut request = Request::default();
        request.hash_search_request = Some(HashSearchRequest {
            search_string: input
        });
        Ok(self.client_wrapper().proto_post_request(&mut request, None).await?.hash_search_response.safe_get()?.clone())
    }
    pub async fn balance(
        &self,
        address: Address,
    ) -> Result<f64, ErrorInfo> {
        let response = self.query_hash(address.render_string().expect("")).await?;
        let ai = response.address_info.safe_get_msg("missing address_info")?;
        Ok(rounded_balance_i64(ai.balance))
    }

    pub async fn address_info(
        &self,
        address: Address,
    ) -> Result<AddressInfo, ErrorInfo> {
        let response = self.query_hash(address.render_string().expect("")).await?;
        let ai = response.address_info.safe_get_msg("missing address_info")?;
        Ok(ai.clone())
    }

    pub async fn about(&self) -> Result<AboutNodeResponse, ErrorInfo> {
        let mut request = empty_public_request();
        request.about_node_request = Some(AboutNodeRequest{ verbose: true });
        let result = self.request(&request).await;
        let result1 = result?.as_error();
        let option = result1?.about_node_response;
        let result2 = option.safe_get_msg("Missing response");
        Ok(result2?.clone())
    }

}
//
// async fn request(t: &Transaction) -> Result<Response, Box<dyn std::error::Error>> {
//     let client = reqwest::Client::new();
//     let res = client
//         .post("http://localhost:3030/request")
//         .json(&t)
//         .send()
//         .await?
//         .json::<Response>()
//         .await?;
//     Ok(res)
// }

// #[test]
// fn test_enum_ser() {
//     State::Pending.
// }

// TODO: wrapper function to handle errors and return as json
// TODO: wrapper function to covnert result to warp json

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

    if let Some(f) = request.faucet_request {
        if let Some(a) = f.address {
            let fr = faucet_request(a.render_string()?, relay.clone()).await?;
            response1.faucet_response = Some(fr);
        }
    }
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

pub async fn handle_proto_post(reqb: Bytes, _address: Option<SocketAddr>, relay: Relay) -> Result<RResponse, ErrorInfo> {
    let vec_b = reqb.to_vec();
    let request = Request::proto_deserialize(vec_b)?;
    // info!{"Warp request from {:?}", address};
    // TODO: increment metric?
    let c = new_channel::<RResponse>();
    let mut msg = PeerMessage::empty();
    msg.request = request;
    msg.response = Some(c.sender.clone());
    relay.peer_message_rx.send(msg).await?;
    c.receiver.recv_async_err_timeout(Duration::from_secs(40)).await
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

    let faucet_relay = relay.clone();

    let faucet = warp::get()
        .and(warp::path("faucet"))
        .and(warp::path::param())
        .and_then(move |address: String| {
            let relay3 = faucet_relay.clone();
            async move {
                let res: Result<Json, warp::reject::Rejection> =
                    Ok(faucet_request(address, relay3.clone()).await
                        .map_err(|e| warp::reply::json(&e))
                        .map(|r| warp::reply::json(&r))
                        .combine()
                    );
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

    let p2_relay = relay.clone();
    let transaction_lookup = warp::get()
        .and(warp::path("transaction"))
        .and(warp::path::param())
        .and_then(move |hash: String| {
            let relay3 = p2_relay.clone();
            async move {
                let ps = relay3.ds.transaction_store
                    .query_transaction_hex(hash).await;
                let res: Result<Json, warp::reject::Rejection> = Ok(ps
                       .map_err(|e| warp::reply::json(&e))
                       .map(|r| warp::reply::json(&r))
                       .combine());
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
                let ps = relay3.peer_id().await;
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

    let tmp_relay = relay.clone();
    let seeds = warp::get()
        .and(warp::path("seeds"))
        .and_then(move || {
            let relay3 = tmp_relay.clone();
            async move {
                let ps: RgResult<Vec<Seed>> = Ok(relay3.node_config.seeds);
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
                    let response = relay3.receive_message_sync(request, None).await;
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
        .and_then(move |reqb: Bytes, address: Option<SocketAddr>| {
            // TODO: verify auth and receive message sync from above
            let relay3 = bin_relay2.clone();
            let result = async move {
                let res: Result<Response<Vec<u8>>, warp::Rejection> =
                    Ok(as_warp_proto_bytes(handle_proto_post(reqb, address, relay3.clone()).await).await);
                res
            };
            result
        });
    //
    // let explorer_relay = relay.clone();
    // let explorer_hash = warp::get()
    //     .and(warp::path("explorer"))
    //     .and(warp::path("hash"))
    //     .and(warp::path::param())
    //     .and(warp::query::<Pagination>())
    //     .and_then(move |hash: String, pagination: Pagination| {
    //         let relay3 = explorer_relay.clone();
    //         async move {
    //             as_warp_json_response( explorer::handle_explorer_hash(hash, relay3.clone(), pagination).await)
    //         }
    //     }).with(warp::cors().allow_any_origin());  // add this line to enable CORS;
    //
    //
    // let explorer_relay2 = relay.clone();
    // let explorer_recent = warp::get()
    //     .and(warp::path("explorer"))
    //     .and_then(move || {
    //         let relay3 = explorer_relay2.clone();
    //         async move {
    //             as_warp_json_response( explorer::handle_explorer_recent(relay3.clone()).await)
    //         }
    //     })
    //     .with(warp::cors().allow_any_origin());  // add this line to enable CORS;

    let port = relay2.node_config.public_port();
    info!("Running public API on port: {:?}", port.clone());

    let routes = hello
        .or(seeds)
        .or(trust)
        .or(peer_tx)
        .or(node_tx)
        .or(peer_id)
        .or(public)
        .or(transaction)
        .or(faucet)
        .or(query_hash)
        .or(about)
        .or(request_normal)
        .or(request_bin)
        .or(peers)
        .or(transaction_lookup)
        .or(address_lookup)
        // .or(explorer_hash)
        // .or(explorer_recent)
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
    Ok(server)
}

#[derive(serde::Deserialize)]
pub struct Pagination {
    pub offset: Option<u32>,
    pub limit: Option<u32>,
}

pub fn start_server(relay: Relay
                    // , runtime: Arc<Runtime>
) -> JoinHandle<Result<(), ErrorInfo>> {

    let handle = tokio::spawn(run_server(relay.clone()));
    info!("Started PublicAPI server on port {:?}", relay.clone().node_config.public_port());
    return handle;
}

#[allow(dead_code)]
async fn mock_relay(relay: Relay) {
    loop {
        let tm = relay.mempool.receiver.recv().unwrap();
        let mut response = structs::Response::default();
        response.submit_transaction_response = Some(SubmitTransactionResponse {
                transaction_hash: create_test_genesis_transaction().hash_or().into(),
                query_transaction_response: Some(QueryTransactionResponse {
                    observation_proofs: vec![]
                }),
            transaction: None,
        });
        tm.response_channel
            .unwrap()
            .send(response)
            .expect("send");
    }
}
//
// #[tokio::test]
// async fn run_warp_debug() {
//     // Only for debug
//     util::init_logger().expect("log");
//     let relay = Relay::default().await;
//     info!("Starting on: {:?}", relay.node_config.public_port);
//     let runtimes = crate::node::NodeRuntimes::default();
//     let res = crate::api::public_api::start_server(relay.clone(), runtimes.public_api.clone());
//     res.await.expect("victree");
//     // run_server(relay).await;
// }

// #[tokio::test]
// #[ignore]
// #[test]
// fn test_warp_basic() {
//     util::init_logger().expect("log");
//
//     let arc2 = build_runtime(2, "test-public-api");
//
//     let runtime = Arc::new(
//         Builder::new_multi_thread()
//             .worker_threads(2)
//             .thread_name("public-api-test")
//             .thread_stack_size(3 * 1024 * 1024)
//             .enable_all()
//             .build()
//             .unwrap(),
//     );
//
//     let mut relay = arc2.block_on(Relay::default());
//     let offset = (3030 + OsRng::default().next_u32() % 40000) as u16;
//     relay.node_config.public_port = Some(offset);
//     start_server(relay.clone(), arc2.clone());
//     runtime
//         .clone()
//         .block_on(async { sleep(Duration::new(3, 0)).await });
//     let res = runtime.clone().block_on(async move {
//         PublicClient::local(offset)
//             .send_transaction(&create_genesis_transaction(), false)
//             .await
//             .unwrap()
//     });
//     let relay_t = relay.transaction.receiver.recv();
//     assert_eq!(create_genesis_transaction(), relay_t.unwrap().transaction);
//     let mut response = empty_public_response();
//     response.submit_transaction_response = Some(SubmitTransactionResponse {
//             transaction_hash: create_genesis_transaction().hash().into(),
//             query_transaction_response: None,
//         transaction: None,
//     });
//     // assert_eq!(
//     //     response,
//     //     res
//     // );
//
//     let res2 = runtime.clone().block_on(async move {
//         PublicClient::local(offset)
//             .query_addresses(vec![])
//             .await
//             .unwrap()
//     });
//     println!("response: {:?}", res2);
// }

//
// #[test]
// fn test_warp_basic2() {
//     util::init_logger().expect("log");
//
//     let arc2 = build_runtime(2, "test-public-api");
//
//     let runtime = Arc::new(
//         Builder::new_multi_thread()
//             .worker_threads(2)
//             .thread_name("public-api-test")
//             .thread_stack_size(3 * 1024 * 1024)
//             .enable_all()
//             .build()
//             .unwrap(),
//     );
//
//     let mut relay = arc2.block_on(Relay::default());
//     let offset = (3030 + OsRng::default().next_u32() % 40000) as u16;
//     relay.node_config.public_port = Some(offset);
//     start_server(relay.clone(), arc2.clone());
//     runtime
//         .clone()
//         .block_on(async { sleep(Duration::new(3, 0)).await });
//     let request = Request::empty().about();
//     let res = runtime.clone().block_on(async move {
//         crate::api::Client::new("localhost".into(), offset)
//             .proto_post(&request, "request_proto".into())
//             .await
//     });
//
//     println!("response: {:?}", res);
// }
//
// #[test]
// fn test_warp_basic3() {
//     util::init_logger().expect("log");
//
//     let arc2 = build_runtime(2, "test-public-api");
//
//     let runtime = Arc::new(
//         Builder::new_multi_thread()
//             .worker_threads(2)
//             .thread_name("public-api-test")
//             .thread_stack_size(3 * 1024 * 1024)
//             .enable_all()
//             .build()
//             .unwrap(),
//     );
//
//     let mut relay = arc2.block_on(Relay::default());
//     let offset = (3030 + OsRng::default().next_u32() % 40000) as u16;
//     relay.node_config.public_port = Some(offset);
//     start_server(relay.clone(), arc2.clone());
//     runtime
//         .clone()
//         .block_on(async { sleep(Duration::new(3, 0)).await });
//     let request = Request::empty().about();
//     let res = runtime.clone().block_on(async move {
//         crate::api::Client::new("localhost".into(), offset)
//             .json_post::<Request, RResponse>(&request, "request_peer".into())
//             .await
//     });
//
//     println!("response: {:?}", res);
// }


//
// #[tokio::test]
// async fn test_warp_basic_timeout() {
//     init_logger();
//     info!("starting test");
//     let mut relay = Relay::default();
//     let offset = (3030 + OsRng::default().next_u32() % 40000) as u16;
//     relay.node_config.public_port = Some(offset);
//     start_server(relay.clone());
//     info!("started server");
//
//     sleep(Duration::new(1, 0)).await;
//     let client = PublicClient::local_timeout(offset, Duration::new(0, 1));
//     info!("starting send");
//     // outer timeout seems not to work properly.
//     let res = timeout(
//         Duration::from_secs(1),
//         client.send_transaction(&create_genesis_transaction(), true),
//     )
//     .await
//     .unwrap();
//     assert!(res.is_err())
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