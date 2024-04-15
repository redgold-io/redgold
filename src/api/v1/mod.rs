use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use async_trait::async_trait;
use futures::TryFutureExt;
use serde::Serialize;
use sha3::digest::generic_array::functional::FunctionalSequence;
use warp::{Filter, Rejection};
use warp::path::Exact;
use warp::reply::{Json, Response};
use redgold_schema::RgResult;
use redgold_schema::structs::Hash;
use crate::api::as_warp_json_response;
use crate::api::explorer::server::{extract_ip, process_origin};
use crate::core::relay::Relay;


#[derive(Clone)]
pub struct ApiData {
    pub relay: Arc<Relay>,
    pub origin_ip: Option<String>,
    pub param: Option<String>
}

pub trait ApiHelpers {
    fn with_relay_and_ip(self, r: Arc<Relay>) -> impl Filter<Extract = (ApiData,), Error = Rejection> + Send + Clone;
}

impl<T: Filter<Extract=(), Error=Rejection> + Sized + Send + Clone> ApiHelpers for T {
    fn with_relay_and_ip(self, r: Arc<Relay>) -> impl Filter<Extract=(ApiData, ), Error=Rejection> + Send + Clone {
        let c = r.clone();
        self
            .and(warp::addr::remote())
            .and(extract_ip())
            .map(move |remote: Option<SocketAddr>, ip_header: Option<String>| {
                let origin = process_origin(remote, ip_header);
                ApiData{
                    relay: c.clone(),
                    origin_ip: origin,
                    param: None,
                }
            })
    }
}


pub trait ApiV1Helpers {
    fn with_v1(self) -> impl Filter<Extract = (), Error = Rejection> + Send + Clone;
}


impl<T: Filter<Extract=(), Error=Rejection> + Copy + Sized + Send + Clone> ApiV1Helpers for T {
    fn with_v1(self) -> impl Filter<Extract=(), Error=Rejection> + Send + Clone {
        self.and(warp::path("v1"))
    }
}

//
// #[async_trait]
// pub trait ApiAndThenHelpers {
//     fn and_then_as<R, F>(
//         &self, func: dyn FnOnce(ApiData) -> dyn Future<Output=RgResult<R>>
//     ) -> impl Filter<Extract = (Response,), Error = Rejection> + Clone
//     where R: Send + Serialize,
//     F: Future<Output=RgResult<R>> + Sized;
// }
//
// #[async_trait]
// impl<T: Filter<Extract=(ApiData,), Error=Rejection> + Copy + Sized> ApiAndThenHelpers for T {
//     fn and_then_as<R, F>(
//         &self, func: impl FnOnce(ApiData) -> dyn Future<Output=RgResult<R>>
//     ) -> impl Filter<Extract = (Response,), Error = Rejection> + Clone
//         where R: Send + Serialize,
//               F: Future<Output=RgResult<R>> + Sized {
//         self.and_then(move |api_data: ApiData| async {
//             let result = func(api_data).await;
//             as_warp_json_response(result)
//         })
//     }
// }
#[async_trait]
pub trait ApiAndThenHelpers {
    fn and_then_as<R, F, Fut>(
        self,
        func: F,
    ) -> impl Filter<Extract = (Json,), Error = Rejection> + Send + Clone
        where
            R: Send + Serialize,
            F: Fn(ApiData) -> Fut + Clone + Send,
            Fut: Future<Output = RgResult<R>> + Send;
}

#[async_trait]
impl<T: Filter<Extract = (ApiData,), Error = Rejection> + Sized + Send + Clone> ApiAndThenHelpers for T {
    fn and_then_as<R, F, Fut>(
        self,
        func: F,
    ) -> impl Filter<Extract = (Json,), Error = Rejection> + Send + Clone
        where
            R: Send + Serialize,
            F: FnOnce(ApiData) -> Fut + Clone + Send ,
            Fut: Future<Output = RgResult<R>> + Send,
    {

        self.and_then(move |api_data: ApiData| {
            let func = func.clone();
            async move {
                let result = func(api_data).await;
                as_warp_json_response(result)
            }
        })
    }
}

// -> impl Filter<Extract=(), Error=Rejection> + Copy + Sized)
pub fn v1_api_routes(r: Arc<Relay>) -> impl Filter<Extract = (impl warp::Reply + 'static,), Error = Rejection> + Clone + Send {

    let hello =
        warp::get()
            .with_v1()
            .and(warp::path("hello"))
            .with_relay_and_ip(r.clone())
            .and_then(|api_data: ApiData| async move {
            // .and_then(|| async move {
            let res = format!("hello {}", api_data.origin_ip.unwrap_or("".to_string()));
            // let res = "hello".to_string();
            Ok::<_, Rejection>(res)
        });


    let table_sizes =
        warp::get()
            .with_v1()
            .and(warp::path("tables"))
            .with_relay_and_ip(r.clone())
            .and_then_as(move |api_data: ApiData| async move {
                api_data.relay.ds.table_sizes().await
            });

    let seeds = warp::get()
        .with_v1()
        .and(warp::path("seeds"))
        .with_relay_and_ip(r.clone())
        .and_then_as(move |api_data: ApiData| async move {
            Ok(api_data.relay.node_config.seeds_now())
        });

    let transaction_get = warp::get()
        .with_v1()
        .and(warp::path("transaction"))
        .with_relay_and_ip(r.clone())
        .and(warp::path::param())
        .map(|mut api_data: ApiData, hash: String| {
            api_data.param = Some(hash);
            api_data
        })
        .and_then_as(move |api_data: ApiData| async move {
            api_data.relay.lookup_transaction_maybe_error_hex(&api_data.param.unwrap()).await
        });



    hello
        .or(table_sizes)
        .or(seeds)
        .or(transaction_get)
}


