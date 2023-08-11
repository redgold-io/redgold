use std::future::Future;
use std::time::Duration;
use bytes::Bytes;
use futures::TryFutureExt;
use log::{error, info};
use warp::{Filter, Rejection};
use redgold_schema::structs::ErrorInfo;
use warp::reply::{Json, WithStatus};
use serde::{Deserialize, Serialize};
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use warp::path::FullPath;
use redgold_schema::{error_message, error_msg, ErrorInfoContext, RgResult, structs};
use crate::api::{RgHttpClient, easy_post, rosetta, with_response_logger, with_response_logger_error};
use crate::api::rosetta::models::{AccountBalanceRequest, AccountBalanceResponse, AccountCoinsRequest, AccountIdentifier, Error};
use crate::api::rosetta::spec::Rosetta;
use crate::core::relay::Relay;
use redgold_schema::util::lang_util::SameResult;
use crate::api::rosetta::handlers::*;
use crate::api::rosetta::models;
use crate::util::random_port;
// use crate::api::rosetta::reject::apply_rejection;


fn format_error<T>(
    result: Result<T, ErrorInfo>
) -> Result<T, Error> {
    result.map_err(|e| {
            let e2 = e.clone();
            Error {
                code: e.code as u32,
                message: e.description,
                description: Some(e.description_extended),
                retriable: e.retriable,
                details: Some(e2),
            }
    })
}


fn format_response<T>(
    result: Result<T, Error>,
) -> WithStatus<Json>
    where
        T: Serialize,
{
    let response = result
        .map_err(|e| {
            warp::reply::with_status(
                warp::reply::json(&e),
                StatusCode::from_u16(500).expect("Status code"),
            )
        })
        .map(|j| {
            warp::reply::with_status(
                warp::reply::json(&j),
                StatusCode::from_u16(200).expect("Status code"),
            )
        })
        .combine();
    response
}

async fn deser<'a, T : Deserialize<'a>>(s: &'a str) -> Result<T, ErrorInfo> {
    serde_json::from_str::<T>(s)
        .error_msg(structs::Error::DeserializationFailure, "error deserializing string input as type")
}

fn ser<T>(t: Result<T, ErrorInfo>, e: String) -> WithStatus<Json>
    where
        T: Serialize + Clone,{
    with_response_logger_error(t.clone(), e).ok();
    let result = format_error(t);
    format_response(result)
}

pub async fn handle_warp_request(path: String, r: Rosetta, b: Bytes) -> Result<WithStatus<Json>, Rejection> {
    let e = path.clone();
    let p = &*path;
    let vec = b.to_vec();
    let so = match std::str::from_utf8(&*vec) {
        Ok(v) => v,
        Err(er) => {
            return Ok(
                ser(Err::<AccountBalanceResponse, ErrorInfo>(
                    error_msg(structs::Error::UnknownError, "Error deserializing bytes to utf8 string", er.to_string())
                ), e.clone())
            );
        },
    };
    let so2 = so.clone();
    let s = so2.to_owned();
    log::debug!("Request ENDPOINT={} BODY_TEXT={}", e.clone(), s.clone());

    let res = match p {
        "/account/balance" => {
            // TODO: abstract as method
            ser(deser(so).and_then(|req| account_balance(r, req)).await, e)
        },
        "/account/coins" => {
            // TODO: abstract as method
            ser(deser(so).and_then(|req| account_coins(r, req)).await, e)
        },
        "/block" => {
            // TODO: abstract as method
            ser(deser(so).and_then(|req| block(r, req)).await, e)
        },
        "/block/transaction" => {
            // TODO: abstract as method
            ser(deser(so).and_then(|req| block_transaction(r, req)).await, e)
        },
        "/call" => {
            // TODO: abstract as method
            ser(deser(so).and_then(|req| call(r, req)).await, e)
        },
        "/construction/combine" => {
            // TODO: abstract as method
            ser(deser(so).and_then(|req| construction_combine(r, req)).await, e)
        },
        "/construction/derive" => {
            // TODO: abstract as method
            ser(deser(so).and_then(|req| construction_derive(r, req)).await, e)
        },
        "/construction/hash" => {
            // TODO: abstract as method
            ser(deser(so).and_then(|req| construction_hash(r, req)).await, e)
        },
        "/construction/metadata" => {
            // TODO: abstract as method
            ser(deser(so).and_then(|req| construction_metadata(r, req)).await, e)
        },
        "/construction/parse" => {
            // TODO: abstract as method
            ser(deser(so).and_then(|req| construction_parse(r, req)).await, e)
        },
        "/construction/payloads" => {
            // TODO: abstract as method
            ser(deser(so).and_then(|req| construction_payloads(r, req)).await, e)
        },
        "/construction/preprocess" => {
            // TODO: abstract as method
            ser(deser(so).and_then(|req| construction_preprocess(r, req)).await, e)
        },
        "/construction/submit" => {
            // TODO: abstract as method
            ser(deser(so).and_then(|req| construction_submit(r, req)).await, e)
        },
        "/events/blocks" => {
            // TODO: abstract as method
            ser(deser(so).and_then(|req| events_blocks(r, req)).await, e)
        },
        "/mempool" => {
            // TODO: abstract as method
            ser(deser(so).and_then(|req| mempool(r, req)).await, e)
        },
        "/mempool/transaction" => {
            // TODO: abstract as method
            ser(deser(so).and_then(|req| mempool_transaction(r, req)).await, e)
        },
        "/network/list" => {
            // TODO: abstract as method
            ser(deser(so).and_then(|req| network_list(r, req)).await, e)
        },
        "/network/options" => {
            // TODO: abstract as method
            ser(deser(so).and_then(|req| network_options(r, req)).await, e)
        },
        "/network/status" => {
            // TODO: abstract as method
            ser(deser(so).and_then(|req| network_status(r, req)).await, e)
        },
        "/search/transactions" => {
            // TODO: abstract as method
            ser(deser(so).and_then(|req| search_transactions(r, req)).await, e)
        },
        x => {
            log::error!("Invalid or unsupported endpoint for rosetta {}", x.to_string());
            ser(Err::<Error, ErrorInfo>(error_message(
                structs::Error::UnknownError, format!("unknown endpoint {}", e))
            ), e)
        }
    };

    Ok(res)
}

pub async fn run_server(relay: Relay) -> RgResult<()> {
    let relay2 = relay.clone();

    let hello = warp::get().and_then(|| async {
        let res: Result<&str, Rejection> = Ok("hello");
        res
    });

    let r = Rosetta {
        relay: relay2.clone(),
    };

    let endpoints = warp::post()
        .and(warp::path::full().map(|path: FullPath| {
            let p = path.as_str().to_string();
            log::info!("warp map func full path {}", p);
            p
        }))
        .map(move |p| (p, r.clone()))
        .untuple_one()
        .and(warp::body::bytes())
        // .untuple_one()
        .and_then(handle_warp_request);

    let port = relay2.node_config.rosetta_port();
    info!("Running rosetta API on port: {:?}", port.clone());
    warp::serve(endpoints.or(hello))
        .run(([0, 0, 0, 0], port))
        .await;
    Ok(())
}



// how to handle full path
// let counts = Arc::new(Mutex::new(HashMap::new()));
// let access_counter = warp::path::full()
//     .map(move |path: FullPath| {
//         let mut counts = counts.lock().unwrap();
//
//         *counts.entry(path.as_str().to_string())
//             .and_modify(|c| *c += 1)
//             .or_insert(0)
//     });
//
// let route = warp::path("foo")
//     .and(warp::path("bar"))
//     .and(access_counter)
//     .map(|count| {
//         format!("This is the {}th visit to this URL!", count)
//     });

// Gave up on this, too complicated but useful example for later maybe
// pub fn post<T, ReqT, RespT, Fut, S>(
//     clonable: T,
//     endpoint: S,
//     handler: fn(T, ReqT) -> Fut,
// ) -> impl Filter<Extract = (WithStatus<Json>,), Error = Rejection> + Clone
//     where T : Clone + Send,
//           ReqT : DeserializeOwned + Send + std::fmt::Debug + ?Sized + Serialize + Clone,
//           RespT : DeserializeOwned + Send + std::fmt::Debug + Serialize + Clone,
//           Fut: Future<Output = Result<Result<RespT, ErrorInfo>, Rejection>> + Send,
//           S: Into<String> + Clone + Send
// {
//     easy_post(clonable, endpoint.clone().into(), handler, 16)
//         .map(|x| format_error(x))
//         .map(move |r| with_response_logger_error(r, endpoint.clone().into()))
//         .map(|r| format_response(r))
// }

