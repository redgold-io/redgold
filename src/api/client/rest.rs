use crate::core::relay::Relay;
use itertools::Itertools;
use redgold_keys::request_support::{RequestSupport, ResponseSupport};
use redgold_keys::word_pass_support::NodeConfigKeyPair;
use redgold_schema::explorer::DetailedAddress;
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::observability::errors::{EnhanceErrorInfo, Loggable};
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{AboutNodeRequest, AboutNodeResponse, Address, AddressInfo, CurrencyAmount, ErrorInfo, GetActivePartyKeyRequest, GetPeersInfoRequest, HashSearchRequest, HashSearchResponse, NetworkEnvironment, PublicKey, Request, Response, Seed, SubmitTransactionRequest, SubmitTransactionResponse, Transaction};
use redgold_schema::util::lang_util::WithMaxLengthString;
use redgold_schema::{error_info, structs, ErrorInfoContext, RgResult, SafeOption};
use reqwest::ClientBuilder;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;
use tracing::debug;
use uuid::Uuid;

#[derive(Clone)]
pub struct RgHttpClient {
    pub url: String,
    pub port: u16,
    pub timeout: Duration,
    pub relay: Option<Relay>,
    pub http_proxy: Option<String>
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
            http_proxy: None,
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

    pub fn from_env(url: String, network_environment: &NetworkEnvironment) -> Self {
        Self {
            url,
            port: network_environment.default_port_offset() + 1,
            timeout: Duration::from_secs(150),
            relay: None,
            http_proxy: None,
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
        let client = ClientBuilder::new()
            .timeout(self.timeout)
            .build().unwrap();
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
        let mut builder = ClientBuilder::new().timeout(self.timeout);
        if let Some(h) = self.http_proxy.as_ref() {
            builder = builder.proxy(reqwest::Proxy::http(h).error_info("Failed to build proxy")?);
        }
        builder.build().error_info("Failed to build client")
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
    pub async fn enriched_party_data(&self) -> HashMap<PublicKey, PartyInternalData> {
        self.party_data().await.log_error().map(|mut r| {
            r.iter_mut().for_each(|(k, v)| {
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