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
use serde::__private::de::Borrowed;
use tracing::info;
use uuid::Uuid;
use warp::reply::Json;
use warp::{Filter, Rejection};
use redgold_keys::request_support::{RequestSupport, ResponseSupport};
use redgold_schema::{EasyJson, error_info, ProtoHashable, ProtoSerde, RgResult, SafeOption, structs};
use redgold_schema::errors::EnhanceErrorInfo;
use redgold_schema::structs::{AboutNodeRequest, AboutNodeResponse, Address, UtxoId, GetPeersInfoRequest, GetPeersInfoResponse, Request, Response, HashSearchResponse, HashSearchRequest, Transaction, PublicKey};
use crate::core::relay::Relay;
use crate::node_config::NodeConfig;
use redgold_schema::util::lang_util::SameResult;

pub mod control_api;
pub mod public_api;
pub mod rosetta;
pub mod faucet;
pub mod hash_query;
pub mod udp_api;
pub mod about;
pub mod explorer;


#[derive(Clone)]
pub struct RgHttpClient {
    pub url: String,
    pub port: u16,
    pub timeout: Duration,
    pub relay: Option<Relay>
}

impl RgHttpClient {
    pub fn new(url: String, port: u16, relay: Option<Relay>) -> Self {
        Self {
            url,
            port,
            timeout: Duration::from_secs(60),
            relay,
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
        self.json_post(r, "request".to_string()).await
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
    pub async fn proto_post<Req: Sized + ProtoSerde>(
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

    pub async fn proto_post_request(&self, mut r: Request, nc: Option<&Relay>, intended_pk: Option<&PublicKey>) -> Result<Response, ErrorInfo> {
        if r.trace_id.is_none() {
            r.trace_id = Some(Uuid::new_v4().to_string());
        }

        let mut r = if let Some(relay) = nc.or(self.relay.as_ref()) {
            let rrr = r.with_metadata(relay.node_metadata().await?)
                .with_auth(&relay.node_config.keypair());
            rrr.verify_auth().add("Self request signing immediate auth failure")?;
            // let h = rrr.calculate_hash();
            // info!("proto_post_request calculate_hash={} after verify auth: {}", h.hex(), rrr.json_or());
            rrr
        } else {
            r
        };
        let result = self.proto_post(&r, "request_proto".to_string()).await?;
        result.as_error_info().add("Response metadata found as errorInfo")?;
        let string = result.json_or();
        result.verify_auth(intended_pk).add("Response authentication verification failure").add(string)
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

    pub async fn get_peers(&self) -> Result<Response, ErrorInfo> {
        let mut req = Request::default();
        req.get_peers_info_request = Some(GetPeersInfoRequest::default());
        let response = self.proto_post_request(req, None, None).await?;
        Ok(response)
    }

    pub async fn contract_state(&self, address: &Address
                                // , utxo_id: &UtxoId
    ) -> RgResult<structs::ContractStateMarker> {
        let mut req = Request::default();
        let mut cmr = structs::GetContractStateMarkerRequest::default();
        // cmr.utxo_id = Some(utxo_id.clone());
        cmr.address = Some(address.clone());
        req.get_contract_state_marker_request = Some(cmr);
        let response = self.proto_post_request(req, None, None).await?;
        Ok(response.get_contract_state_marker_response.ok_or(error_info("Missing get_contract_state_marker_response"))?)
    }

    pub async fn about(&self) -> RgResult<AboutNodeResponse> {
        let mut req = Request::default();
        req.about_node_request = Some(AboutNodeRequest::default());
        let response = self.proto_post_request(req, None, None).await?;
        Ok(response.about_node_response.ok_or(error_info("Missing about node response"))?)
    }

    pub async fn resolve_code(&self, address: &Address) -> RgResult<structs::ResolveCodeResponse> {
        let mut req = Request::default();
        req.resolve_code_request = Some(address.clone());
        let response = self.proto_post_request(req, None, None).await?;
        Ok(response.resolve_code_response.ok_or(error_info("Missing resolve code response"))?)
    }

    pub async fn genesis(&self) -> RgResult<Transaction> {
        let mut req = Request::default();
        req.genesis_request = Some(structs::GenesisRequest::default());
        let response = self.proto_post_request(req, None, None).await?;
        response.genesis_response.ok_msg("Missing genesis response")
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
        Ok(self.proto_post_request(request, None, None).await?.hash_search_response.safe_get()?.clone())
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
            let ser = serde_json::to_string(&y2).unwrap_or("request ser failed".to_string());
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
    let ser = serde_json::to_string(&y2).unwrap_or("response ser failed".to_string());
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
    let ser = serde_json::to_string( & y2).unwrap_or("response ser failed".to_string());
    log::debug ! ("Response success endpoint: {} {} ", endpoint.clone(), ser);
    r
    }).map_err(move |r| {
    let y2 = r.clone();
    let ser = serde_json::to_string( & y2).unwrap_or("response ser failed".to_string());
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

