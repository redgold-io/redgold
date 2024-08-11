use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use async_trait::async_trait;
use futures::TryFutureExt;
use itertools::Itertools;
use serde::Serialize;
use sha3::digest::generic_array::functional::FunctionalSequence;
use warp::{Filter, Rejection};
use warp::path::Exact;
use warp::reply::{Json, Response};
use redgold_keys::address_support::AddressSupport;
use redgold_keys::proof_support::PublicKeySupport;
use redgold_keys::public_key_parse_support::PublicKeyParseSupport;
use redgold_schema::RgResult;
use redgold_schema::structs::{CurrencyAmount, Hash, SupportedCurrency};
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

    let genesis = warp::get()
        .with_v1()
        .and(warp::path("genesis"))
        .with_relay_and_ip(r.clone())
        .and_then_as(move |api_data: ApiData| async move {
            api_data.relay.ds.config_store.get_genesis().await
        });

    let party_data = warp::get()
        .with_v1()
        .and(warp::path("party"))
        .and(warp::path("data"))
        .with_relay_and_ip(r.clone())
        .and_then_as(move |api_data: ApiData| async move {
            let data = api_data.relay.external_network_shared_data.clone_read().await;
            data.iter().map(|(k, v)| {
                let mut v = v.clone();
                v.clear_sensitive();
                v
            }).collect_vec()
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


    // TODO: Waterfall function, from address / raw address / public key proto / compact public key /
    let balance_get = warp::get()
        .with_v1()
        .and(warp::path("balance"))
        .with_relay_and_ip(r.clone())
        .and(warp::path::param())
        .map(|mut api_data: ApiData, hash: String| {
            api_data.param = Some(hash);
            api_data
        })
        .and_then_as(move |api_data: ApiData| async move {
            balance_lookup(api_data.relay, api_data.param.unwrap().clone()).await
        });



    hello
        .or(table_sizes)
        .or(seeds)
        .or(genesis)
        .or(party_data)
        .or(transaction_get)

}

async fn balance_lookup(relay: Arc<Relay>, hash: String) -> RgResult<CurrencyAmount> {
    let net = relay.node_config.network.clone();
    let pk_parse = hash.clone().parse_public_key().and_then(|pk| pk.to_all_addresses_for_network(&net));
    let addrs = hash.clone().parse_address_incl_raw().map(|a| vec![a])
        .or(pk_parse)?;

    let mut total = CurrencyAmount::zero(SupportedCurrency::Redgold);

    for addr in addrs {
        let b = relay.ds.transaction_store.get_balance(&addr).await?;
        if let Some(b) = b {
            total = total + CurrencyAmount::from_rdg(b)
        }
    }
    Ok(total)
}


