use redgold_schema::message::{Request, Response};
use redgold_schema::{error_info, structs, ErrorInfoContext, RgResult, SafeOption};
use redgold_schema::structs::{AboutNodeRequest, AboutNodeResponse, Address, AddressInfo, CurrencyAmount, ErrorInfo, GetActivePartyKeyRequest, GetPeersInfoRequest, HashSearchRequest, HashSearchResponse, NetworkEnvironment, NodeMetadata, PublicKey, Seed, SubmitTransactionRequest, SubmitTransactionResponse, Transaction};
use std::time::Duration;
use redgold_schema::explorer::DetailedAddress;
use std::collections::HashMap;
use reqwest::ClientBuilder;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::debug;
use uuid::Uuid;
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::observability::errors::{EnhanceErrorInfo, Loggable};
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::util::lang_util::WithMaxLengthString;


pub trait RequestResponseAuth: Send + Sync  {
    fn sign_request(&self, r: &Request) -> RgResult<Request>;
    fn verify(&self, response: Response, intended_pk: Option<&PublicKey>) -> RgResult<Response>;

    // Add this instead of deriving Clone
    fn clone_box(&self) -> Box<dyn RequestResponseAuth>;
}

// 2. Implement clone for boxed trait object
impl Clone for Box<dyn RequestResponseAuth> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}


#[derive(Clone)]
pub struct RgHttpClient {
    pub url: String,
    pub port: u16,
    pub timeout: Duration,
    pub http_proxy: Option<String>,
    pub auth: Option<Box<dyn RequestResponseAuth>>
}

impl RgHttpClient {
    pub async fn address_info_for_pk(&self, p0: &PublicKey) -> RgResult<AddressInfo> {
        let mut req = Request::default();
        req.get_address_info_public_key_request = Some(p0.clone());
        let resp = self.proto_post_request(req, None, None).await?;
        resp.get_address_info_public_key_response.ok_or(error_info("Missing get_address_info_response"))
    }
}

impl RgHttpClient {
    pub fn new(url: String, port: u16, signer: Option<Box<dyn RequestResponseAuth>>) -> Self {
        Self {
            url,
            port,
            timeout: Duration::from_secs(150),
            http_proxy: None,
            auth: signer,
        }
    }

    pub fn with_http_proxy(mut self, http_proxy: String) -> Self {
        self.http_proxy = Some(http_proxy);
        self
    }


    pub async fn address_info(
        &self,
        address: Address,
    ) -> Result<AddressInfo, ErrorInfo> {
        let response = self.query_hash(address.render_string().expect("")).await?;
        let ai = response.address_info.safe_get_msg("missing address_info")?;
        Ok(ai.clone())
    }

    pub async fn send_transaction(
        &self,
        t: &Transaction,
        sync: bool,
    ) -> Result<SubmitTransactionResponse, ErrorInfo> {

        let mut c = self.clone();
        c.timeout = Duration::from_secs(180);

        let mut request = Request::default();
        request.submit_transaction_request = Some(SubmitTransactionRequest {
            transaction: Some(t.clone()),
            sync_query_response: sync,
        });
        debug!("Sending transaction: {}", t.clone().hash_hex());
        let response = c.proto_post_request(request, None, None).await?;
        response.as_error_info()?;
        Ok(response.submit_transaction_response.safe_get()?.clone())
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

    pub fn from_env(url: String, network_environment: &NetworkEnvironment, signer: Option<Box<dyn RequestResponseAuth>>) -> Self {
        Self {
            url,
            port: network_environment.default_port_offset() + 1,
            timeout: Duration::from_secs(150),
            http_proxy: None,
            auth: signer,
        }
    }

    #[allow(dead_code)]
    fn formatted_url(&self) -> String {
        return "http://".to_owned() + &*self.url.clone() + ":" + &*self.port.to_string();
    }

    fn metrics_url(&self) -> String {
        format!("http://{}:{}/metrics", self.url, self.port - 2)
    }

    #[cfg(not(target_arch = "wasm32"))]
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
            .map(|x| x.collect::<Vec<&str>>())
            .flat_map(|x| x.get(0).as_ref().and_then(|k| x.get(1).as_ref().map(|v| (k.to_string(), v.to_string()))))
            // .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<Vec<(String, String)>>();
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

    #[cfg(not(target_arch = "wasm32"))]
    #[allow(dead_code)]
    pub async fn json_post<Req: Serialize + ?Sized, Resp: DeserializeOwned>(
        &self,
        r: &Req,
        endpoint: String,
    ) -> Result<Resp, ErrorInfo> {
        self.client()?.post(format!("{}/{}", self.formatted_url(), endpoint))
            .json::<Req>(r)
            .send().await.error_info("error")?.text().await.error_info("error")?.json_from()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn client(&self) -> RgResult<reqwest::Client> {
        let mut builder = ClientBuilder::new().timeout(self.timeout);
        if let Some(h) = self.http_proxy.as_ref() {
            builder = builder.proxy(reqwest::Proxy::http(h).error_info("Failed to build proxy")?);
        }
        builder.build().error_info("Failed to build client")
    }

    #[cfg(not(target_arch = "wasm32"))]
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

    #[cfg(target_arch = "wasm32")]
    pub async fn proto_post<Req: Sized + ProtoSerde>(
        &self,
        r: &Req,
        endpoint: String,
    ) -> Result<Response, ErrorInfo> {
        "not".to_error()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn proto_post<Req: Sized + ProtoSerde>(
        &self,
        r: &Req,
        endpoint: String,
    ) -> Result<Response, ErrorInfo> {
        let sent = self.client()?
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


    #[cfg(target_arch = "wasm32")]
    pub async fn proto_post_request(
        &self,
        mut r: Request,
        nmd: Option<NodeMetadata>,
        intended_pk: Option<&PublicKey>
    ) -> Result<Response, ErrorInfo> {
        "not".to_error()
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn proto_post_request(
        &self,
        mut r: Request,
        nmd: Option<NodeMetadata>,
        intended_pk: Option<&PublicKey>
    ) -> Result<Response, ErrorInfo> {
        if r.trace_id.is_none() {
            r.trace_id = Some(Uuid::new_v4().to_string());
        }

        if let Some(nmd) = nmd {
            r = r.with_metadata(nmd)
        };
        if let Some(signer) = self.auth.as_ref() {
            r = signer.sign_request(&r)?;
        }
        let result = self.proto_post(&r, "request_proto".to_string()).await?;
        result.as_error_info().add("Response metadata found as errorInfo")?;
        let string = result.json_or();
        if let Some(signer) = self.auth.as_ref() {
            signer.verify(result, intended_pk).add("Response authentication verification failure").add(string.with_max_length(1000))
        } else {
            Ok(result)
        }
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
            hm.insert(pd.proposer_key.clone(), pd);
        }
        Ok(hm)
    }
    pub async fn enriched_party_data(&self) -> HashMap<PublicKey, PartyInternalData> {
        self.party_data().await.log_error().map(|mut r| {
            r.iter_mut().for_each(|(_, v)| {
                v.party_events.as_mut().map(|pev| {
                    pev.portfolio_request_events.enriched_events = Some(pev.portfolio_request_events.calculate_current_fulfillment_by_event());
                });
            });
            r.clone()
        }).unwrap_or_default()
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