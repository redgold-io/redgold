use std::collections::HashMap;
use std::net::SocketAddr;
use futures::TryFutureExt;
use itertools::Itertools;
use log::info;
use tokio::task::JoinHandle;
use warp::Filter;
use redgold_keys::address_support::AddressSupport;
use redgold_schema::structs::{Address, ErrorInfo, FaucetRequest, Request};
use crate::api::{as_warp_json_response, explorer};
use crate::api::explorer::{handle_explorer_faucet, handle_explorer_pool};
use crate::api::public_api::{TokenParam, Pagination};
use crate::core::relay::Relay;


pub fn start_server(relay: Relay) -> JoinHandle<Result<(), ErrorInfo>> {

    let handle = tokio::spawn(run_server(relay.clone()));
    info!("Started Explorer API server on port {:?}", relay.clone().node_config.explorer_port());
    return handle;
}

fn extract_ip() -> impl Filter<Extract = (Option<String>,), Error = warp::Rejection> + Copy {
    warp::header::optional("X-Real-IP")
        .or(warp::header::optional("X-Forwarded-For"))
        .unify()
}

pub fn allowed_proxy_origins() -> Vec<String> {
    vec![
        "209.159.152.2"
    ].iter().map(|x| x.to_string()).collect_vec()
}

fn process_origin(socket: Option<SocketAddr>, remote: Option<String>) -> Option<String> {
    if let Some(socket) = socket {
        let socket_ip = socket.ip().to_string();
        if allowed_proxy_origins().contains(&socket_ip) {
            remote
        } else {
            Some(socket_ip)
        }
    } else {
        None
    }
}

pub async fn run_server(relay: Relay) -> Result<(), ErrorInfo>{
    let relay2 = relay.clone();

    let home = warp::get()
        .and_then(|| async move {
            let res: Result<&str, warp::reject::Rejection> = Ok("hello");
            res
        });

    let explorer_relay = relay.clone();
    let explorer_hash = warp::get()
        .and(warp::path("explorer"))
        .and(warp::path("hash"))
        .and(warp::path::param())
        .and(warp::query::<Pagination>())
        .and_then(move |hash: String, pagination: Pagination| {
            let relay3 = explorer_relay.clone();
            async move {
                as_warp_json_response( explorer::handle_explorer_hash(hash, relay3.clone(), pagination).await)
            }
        }).with(warp::cors().allow_any_origin());  // add this line to enable CORS;

    let explorer_relay3 = relay.clone();
    let explorer_faucet = warp::get()
        .and(warp::path("explorer"))
        .and(warp::path("faucet"))
        .and(warp::path::param())
        .and(warp::query::<TokenParam>())
        .and(warp::addr::remote())
        .and(extract_ip())
        .and_then(move |address: String, pagination: TokenParam, remote: Option<SocketAddr>, ip_header: Option<String>| {
            let relay3 = explorer_relay3.clone();
            let origin = process_origin(remote, ip_header);
            async move {
                as_warp_json_response(
                    handle_explorer_faucet(address, relay3, pagination, origin).await
                )
            }
        }).with(warp::cors().allow_any_origin());  // add this line to enable CORS;

    let explorer_relay4 = relay.clone();
    let explorer_pools = warp::get()
        .and(warp::path("explorer"))
        .and(warp::path("pools"))
        .and_then(move || {
            let relay3 = explorer_relay4.clone();
            async move {
                as_warp_json_response(
                    handle_explorer_pool(relay3).await
                )
            }
        }).with(warp::cors().allow_any_origin());  // add this line to enable CORS;

    let explorer_relay2 = relay.clone();
    let explorer_recent = warp::get()
        .and(warp::path("explorer"))
        // Add optional query parameter `is_test`
        .and(warp::query::<HashMap<String, String>>())
        .and_then(move |query_params: HashMap<String, String>| {
            let relay3 = explorer_relay2.clone();
            async move {
                // Extract `is_test` parameter and convert to boolean, defaulting to false if not provided
                let is_test = query_params.get("is_test").map(|value| value == "true");
                as_warp_json_response(explorer::handle_explorer_recent(relay3.clone(), is_test).await)
            }
        })
        .with(warp::cors().allow_any_origin());
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

    let explorer_relay3 = relay.clone();
    let explorer_swap = warp::get()
        .and(warp::path("explorer"))
        .and(warp::path("swap"))
        .and_then(move || {
            let relay3 = explorer_relay3.clone();
            async move {
                as_warp_json_response( explorer::handle_explorer_swap(relay3.clone()).await)
            }
        })
        .with(warp::cors().allow_any_origin());  // add this line to enable CORS;

    let port = relay2.node_config.explorer_port();
    info!("Running explorer API on port: {:?}", port.clone());

    let folder = relay2.node_config.data_folder.all();

    let cert = if let (Ok(cert), Ok(key)) = (folder.cert().await, folder.key().await) {
        Some((cert, key))
    } else {
        info!("Unable to find explorer TLS / SSL cert in: {}", folder.path.to_str().unwrap().to_string());
        None
    };

    let routes = explorer_hash
        .or(explorer_swap)
        .or(explorer_faucet)
        .or(explorer_pools)
        .or(explorer_recent)
        .or(home);

    // Create a warp Service using the filter
    // Create the server
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let server = if let Some((cert, key)) = cert {
        info!("Using SSL/TLS on explorer API");
        warp::serve(routes)
            .tls()
            .cert(cert)
            .key(key)
            .run(addr)
            .await
    } else {
        warp::serve(routes)
            .run(addr)
            .await
    };
    Ok(server)
}
