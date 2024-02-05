use std::collections::HashMap;
use std::net::SocketAddr;
use log::info;
use tokio::task::JoinHandle;
use warp::Filter;
use redgold_schema::structs::ErrorInfo;
use crate::api::{as_warp_json_response, explorer};
use crate::api::public_api::Pagination;
use crate::core::relay::Relay;


pub fn start_server(relay: Relay) -> JoinHandle<Result<(), ErrorInfo>> {

    let handle = tokio::spawn(run_server(relay.clone()));
    info!("Started Explorer API server on port {:?}", relay.clone().node_config.explorer_port());
    return handle;
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
