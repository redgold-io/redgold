use std::borrow::Borrow;
use std::collections::HashMap;
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
use reqwest::ClientBuilder;
use serde::__private::de::Borrowed;
use tracing::info;
use uuid::Uuid;
use warp::reply::Json;
use warp::{Filter, Rejection};
use redgold_keys::request_support::{RequestSupport, ResponseSupport};
use redgold_schema::{empty_public_request, error_info, structs, ErrorInfoContext, RgResult, SafeOption};
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::proto_serde::{ProtoHashable, ProtoSerde};
use redgold_schema::structs::{AboutNodeRequest, AboutNodeResponse, Address, AddressInfo, CurrencyAmount, GetActivePartyKeyRequest, GetPeersInfoRequest, GetPeersInfoResponse, HashSearchRequest, HashSearchResponse, NetworkEnvironment, PublicKey, PublicResponse, QueryAddressesRequest, Request, Response, Seed, Transaction, UtxoId};
use redgold_schema::transaction::rounded_balance_i64;
use crate::core::relay::Relay;
use crate::node_config::NodeConfigKeyPair;
use redgold_schema::util::lang_util::{SameResult, WithMaxLengthString};
use crate::integrations::external_network_resources::ExternalNetworkResourcesImpl;
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::explorer::DetailedAddress;

pub mod control_api;
pub mod public_api;
pub mod rosetta;
pub mod faucet;
pub mod hash_query;
pub mod udp_api;
pub mod about;
pub mod explorer;
pub mod v1;
pub mod udp_keepalive;

#[derive(Clone)]
pub struct RgHttpClient {
    pub url: String,
    pub port: u16,
    pub timeout: Duration,
    pub relay: Option<Relay>
}

impl RgHttpClient {
    pub(crate) async fn address_info_for_pk(&self, p0: &PublicKey) -> RgResult<AddressInfo> {
        let mut req = Request::default();
        req.get_address_info_public_key_request = Some(p0.clone());
        let resp = self.proto_post_request(req, None, None).await?;
        resp.get_address_info_public_key_response.ok_or(error_info("Missing get_address_info_response"))
    }
}

impl RgHttpClient {
    pub fn new(url: String, port: u16, relay: Option<Relay>) -> Self {
        Self {
            url,
            port,
            timeout: Duration::from_secs(150),
            relay,
        }
    }



    pub async fn balance(
        &self,
        address: &Address,
    ) -> Result<i64, ErrorInfo> {
        let response = self.query_hash(address.render_string().expect("")).await?;
        let ai = response.address_info.safe_get_msg("missing address_info")?;
        Ok(ai.balance)
    }


    pub fn url(&self) -> String {
        format!("{}:{}", self.url, self.port)
    }

    pub fn from_env(url: String, network_environment: &NetworkEnvironment) -> Self {
        Self {
            url,
            port: network_environment.default_port_offset() + 1,
            timeout: Duration::from_secs(150),
            relay: None,
        }
    }

    #[allow(dead_code)]
    fn formatted_url(&self) -> String {
        return "http://".to_owned() + &*self.url.clone() + ":" + &*self.port.to_string();
    }

    fn metrics_url(&self) -> String {
        format!("http://{}:{}/metrics", self.url, self.port - 2)
    }

    pub async fn metrics(&self) -> RgResult<Vec<(String, String)>>  {
        let client = ClientBuilder::new().timeout(self.timeout).build().unwrap();
        let sent = client
            .get(self.metrics_url())
            .send();
        let response = sent.await.map_err(|e | error_info(e.to_string()))?;
        let x = response.text().await;
        let text = x.map_err(|e | error_info(e.to_string()))?;
        let res = text.split("\n")
            .filter(|x| !x.starts_with("#"))
            .filter(|x| !x.trim().is_empty())
            .map(|x| x.split(" "))
            .map(|x| x.collect_vec())
            .flat_map(|x| x.get(0).as_ref().and_then(|k| x.get(1).as_ref().map(|v| (k.to_string(), v.to_string()))))
            // .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect_vec();
        Ok(res)
    }

    pub async fn table_sizes(&self) -> RgResult<Vec<(String, i64)>> {
        self.json_get("v1/tables").await
    }

    pub async fn explorer_public_address(&self, pk: &PublicKey) -> RgResult<Vec<DetailedAddress>> {
        self.json_get(format!("v1/explorer/public/address/{}", pk.hex())).await
    }

    pub async fn table_sizes_map(&self) -> RgResult<HashMap<String, i64>> {
        self.table_sizes().await.map(|v| v.into_iter().collect())
    }
    pub async fn metrics_map(&self) -> RgResult<HashMap<String, String>> {
        self.metrics().await.map(|v| v.into_iter().collect())
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

    pub fn client(&self) -> RgResult<reqwest::Client> {
        ClientBuilder::new().timeout(self.timeout).build().error_info("Failed to build client")
    }

    pub async fn json_get<Resp: DeserializeOwned>(
        &self,
        endpoint: impl Into<String>,
    ) -> RgResult<Resp> {
        self.client()?
            .get(format!("{}/{}", self.formatted_url(), endpoint.into()))
            .send()
            .await
            .error_info("Failed to send get request")?
            .text().await.error_info("Failed to get response text")?
            .json_from::<Resp>()
    }

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
            format!("Proto request failure: {}", e.to_string())))
            .with_detail("url", self.url.clone())
            .with_detail("port", self.port.clone().to_string())?;
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

        let r = if let Some(relay) = nc.or(self.relay.as_ref()) {
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
        result.verify_auth(intended_pk).add("Response authentication verification failure").add(string.with_max_length(1000))
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

    pub async fn seeds(&self) -> RgResult<Vec<Seed>> {
        let mut req = Request::default();
        req.get_seeds_request = Some(structs::GetSeedsRequest::default());
        let response = self.proto_post_request(req, None, None).await?;
        Ok(response.get_seeds_response.clone())
    }

    pub async fn active_party_key(&self) -> RgResult<PublicKey> {
        let mut req = Request::default();
        req.get_active_party_key_request = Some(GetActivePartyKeyRequest::default());
        let response = self.proto_post_request(req, None, None).await?;
        Ok(response.get_active_party_key_response.ok_or(error_info("Missing get_active_party_key_response response"))?)
    }

    pub async fn balance_pk(&self, pk: &PublicKey) -> RgResult<CurrencyAmount> {
        let mut req = Request::default();
        req.get_public_key_balance_request = Some(pk.clone());
        let response = self.proto_post_request(req, None, None).await?;
        Ok(response.get_public_key_balance_response.ok_or(error_info("Missing get_public_key_balance_response response"))?)
    }

    pub async fn party_data(&self) -> RgResult<HashMap<PublicKey, PartyInternalData>> {
        let pid = self.json_get::<Vec<PartyInternalData>>("v1/party/data").await?;
        let mut hm = HashMap::new();
        for pd in pid {
            if let Some(k) = pd.party_info.party_key.as_ref() {
                hm.insert(k.clone(), pd);
            }
        }
        Ok(hm)
    }

    pub async fn executable_checksum(&self) -> RgResult<String> {
        let abt = self.about().await?;
        let latest = abt.latest_node_metadata.safe_get_msg("Missing about node metadata latest node metadata")?;
        let checksum = latest.node_metadata()?.version_info.map(|v| v.executable_checksum.clone());
        checksum.safe_get_msg("Missing executable checksum").cloned()
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

    // #[allow(dead_code)]
    // pub async fn address_utxos(
    //     &self,
    //     addresses: Vec<Address>,
    // ) -> Result<PublicResponse, ErrorInfo> {
    //     let mut request = Request::default();
    //     request.query_addresses_request = Some(QueryAddressesRequest {
    //         addresses
    //     });
    //     self.proto_post_request(&request).await
    // }


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

