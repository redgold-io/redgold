use std::borrow::Borrow;
use std::convert::Infallible;
use crate::schema::structs::ErrorInfo;
use crate::util;
use futures::future::AndThen;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use itertools::Itertools;
use prost::Message;
use serde::__private::de::Borrowed;
use warp::reply::Json;
use warp::{Filter, Rejection};
use redgold_schema::{error_info, ProtoHashable};
use redgold_schema::structs::Response;
use crate::util::lang_util::SameResult;

pub mod control_api;
#[cfg(not(target_arch = "wasm32"))]
pub mod p2p_io;
pub mod public_api;
pub mod rosetta;
pub mod faucet;
pub mod lp2p;
pub mod hash_query;
pub mod udp_api;
pub mod about;


#[derive(Clone)]
pub struct HTTPClient {
    pub url: String,
    pub port: u16,
    pub timeout: Duration,
}

impl HTTPClient {
    pub fn new(url: String, port: u16) -> Self {
        Self {
            url,
            port,
            timeout: Duration::from_secs(60),
        }
    }
    #[allow(dead_code)]
    fn formatted_url(&self) -> String {
        return "http://".to_owned() + &*self.url.clone() + ":" + &*self.port.to_string();
    }

    #[allow(dead_code)]
    pub async fn json_post_request<Req: Serialize + ?Sized, Resp: DeserializeOwned>(
        &self,
        r: &Req,
    ) -> Result<Resp, ErrorInfo> {
        self.json_post(r, "request".into()).await
    }

    #[allow(dead_code)]
    pub async fn json_post<Req: Serialize + ?Sized, Resp: DeserializeOwned>(
        &self,
        r: &Req,
        endpoint: String,
    ) -> Result<Resp, ErrorInfo> {
        use reqwest::ClientBuilder;
        let client = ClientBuilder::new().timeout(self.timeout).build().unwrap();
        let sent = client
            .post(format!("{}/{}", self.formatted_url(), endpoint))
            .json::<Req>(r)
            .send();
        let response = sent.await;
        match response {
            Ok(r) => {
                let text = r.text().await
                    .map_err(|e| error_info(format!("{} {}", "Failed to get response text ", e.to_string())))?;
                let resp = serde_json::from_str::<Resp>(&*text.clone())
                    .map_err(|e| error_info(format!("{} {}", e.to_string(), text)))?;
                Ok(resp)
            },
            Err(e) => Err(error_info(e.to_string())),
        }
    }
    #[allow(dead_code)]
    pub async fn proto_post<Req: Sized + Message>(
        &self,
        r: &Req,
        endpoint: String,
    ) -> Result<Response, ErrorInfo> {
        use reqwest::ClientBuilder;
        let client = ClientBuilder::new().timeout(self.timeout).build().unwrap();
        let sent = client
            .post(format!("{}/{}", self.formatted_url(), endpoint))
            .body(r.encode_to_vec())
            .send();
        let response = sent.await.map_err(|e| ErrorInfo::error_info(
            format!("Proto request failure: {}", e.to_string())))?;
        let bytes = response.bytes().await.map_err(|e| ErrorInfo::error_info(
            format!("Proto request bytes decode failure: {}", e.to_string())))?;
        let vec = bytes.to_vec();
        let deser = Response::deserialize(vec).map_err(|e| ErrorInfo::error_info(
            format!("Proto request response decode failure: {}", e.to_string())))?;
        Ok(deser)
    }

    pub async fn test_request<Req, Resp>(port: u16, req: &Req, endpoint: String) -> Result<Resp, ErrorInfo>
    where
        Req: Serialize + ?Sized,
        Resp: DeserializeOwned
    {
        let client = HTTPClient::new("localhost".into(), port);
        tokio::time::sleep(Duration::from_secs(2)).await;
        client.json_post::<Req, Resp>(&req, endpoint).await
    }

}
//
// pub fn endpoint<T, C, R>(
//     path: &str,
//     c: C,
//     f: fn((C, T)) -> dyn Future<Output=Result<R, ErrorInfo>>,
//     fmt: fn(Result<R, ErrorInfo>) -> Result<Json, warp::reject::Rejection>,
// ) -> ()
// where
//     T: DeserializeOwned + Send + Sized,
//     C: Clone + Sized,
//     R: Serialize + Sized,
// {
//     let post = warp::post()
//         .and(warp::path(path))
//         // Only accept bodies smaller than 16kb...
//         .and(warp::body::content_length_limit(1024 * 16))
//         .and(warp::body::json::<T>())
//         .and_then(move |request: T| {
//             let cc = c.clone();
//             async move {
//                 let result: Result<R, ErrorInfo> = f((cc, request)).await;
//                 fmt(result)
//             }
//         });
//     post;
//
// }
//
// // TODO: Gotta be a better way to do this.
// pub fn with_path_inner1(
//     p: Vec<String>
// ) -> impl Filter<Extract = (), Error = Rejection> + Clone + Filter {
//     warp::any()
//         .and(warp::path(p.get(0).expect("0")))
// }
//
// pub fn with_path_inner2(
//     p: Vec<String>
// ) -> impl Filter<Extract = (), Error = Rejection> + Clone + Filter {
//     warp::log::custom()
//     warp::any()
//         .and(warp::path(p.get(0).expect("0")))
//         .and(warp::path(p.get(1).expect("0")))
// }
//
// pub fn with_path_inner(
//     endpoint: String
// ) -> impl Filter<Extract = (), Error = Rejection> + Clone + Filter {
//     let p = endpoint.split("/").collect_vec().iter().map(|s| s.to_string())
//         .collect_vec();
//     match p.len() {
//         1 => {
//             with_path_inner1(p.clone())
//         },
//         2 => {
//             with_path_inner2(p.clone())
//         },
//         _ => {panic!("Bad endpoint path length! {}", endpoint)}
//     }
// }


pub fn easy_post<T, ReqT, RespT, Fut, S>(
    clonable: T,
    endpoint: S,
    handler: fn(T, ReqT) -> Fut,
    length_limit_kb: u64
) -> impl Filter<Extract = (Result<RespT, ErrorInfo>,), Error = Rejection> + Clone
    where T : Clone + Send,
          ReqT : DeserializeOwned + Send + std::fmt::Debug + ?Sized + Serialize + Clone,
          RespT : DeserializeOwned + Send + std::fmt::Debug,
          Fut: Future<Output = Result<Result<RespT, ErrorInfo>, Rejection>> + Send,
          S: Into<String> + Clone + Send
{
    warp::post()
        .and(warp::path !("account" / "balance"))
        // .and(with_path_inner(endpoint.clone().into()))
        .and(warp::body::content_length_limit(1024 * length_limit_kb))
        .map(move || clonable.clone())
        .and(warp::body::json::<ReqT>())
        .map(move |x, y: ReqT| {
            let y2 = y.clone();
            let ser = serde_json::to_string(&y2).unwrap_or("request ser failed".into());
            log::debug!("Request endpoint: {} {} ", endpoint.clone().into(), ser);
            (x, y)
        })
        .untuple_one()
        .and_then(handler)
}
//
// pub fn with_response_logger<Resp>(resp: Resp, endpoint: String) -> impl Filter<Extract = (Resp,), Error = Infallible> + Clone
// where Resp: ?Sized + Serialize + Clone,{
//     warp::any().map(move |resp: Resp| {
//         let y2 = resp.clone();
//         let ser = serde_json::to_string(&y2).unwrap_or("response ser failed".into());
//         log::debug!("Response {}: {:?} ", endpoint.clone(), ser);
//         resp
//     })
// }

pub fn with_response_logger<Resp>(resp: Resp, endpoint: String) -> Resp
where Resp: ?Sized + Serialize + Clone {
    let y2 = resp.clone();
    let ser = serde_json::to_string(&y2).unwrap_or("response ser failed".into());
    log::debug!("Response endpoint: {} {} ", endpoint.clone(), ser);
    resp
}

pub fn with_response_logger_error<Resp, ErrT>(resp: Result<Resp, ErrT>, endpoint_i: String) -> Result<Resp, ErrT>
where Resp: ?Sized + Serialize + Clone,
ErrT: ?Sized + Serialize + Clone {
    let endpoint = endpoint_i.clone();
    let endpoint2 = endpoint_i.clone();
    resp.map(move |r| {
    let y2 = r.clone();
    let ser = serde_json::to_string( & y2).unwrap_or("response ser failed".into());
    log::debug ! ("Response success endpoint: {} {} ", endpoint.clone(), ser);
    r
    }).map_err(move |r| {
    let y2 = r.clone();
    let ser = serde_json::to_string( & y2).unwrap_or("response ser failed".into());
    log::debug ! ("Response error endpoint: {} {} ", endpoint2.clone(), ser);
    r
    })
}


// TODO: implement as trait on result
pub fn as_warp_json_response<T: Serialize, E: Serialize>(response: Result<T, E>) -> Result<Json, warp::reject::Rejection> {
    Ok(response.map_err(|e| warp::reply::json(&e))
    .map(|r| warp::reply::json(&r))
    .combine()
    )
}

