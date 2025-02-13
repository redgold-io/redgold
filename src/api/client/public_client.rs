// TODO: Remove this

use crate::api::client::rest;
use crate::core::relay::Relay;
use crate::schema;
use redgold_schema::helpers::easy_json::json;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::structs::{AboutNodeRequest, AboutNodeResponse, Address, ErrorInfo, FaucetRequest, FaucetResponse, HashSearchRequest, HashSearchResponse, PublicRequest, PublicResponse, QueryAddressesRequest, SubmitTransactionRequest, SubmitTransactionResponse, Transaction};
use redgold_schema::message::Request;
use redgold_schema::{empty_public_request, SafeOption};
use std::time::Duration;
use tracing::{debug, info};

#[derive(Clone)]
pub struct PublicClient {
    pub url: String,
    pub port: u16,
    pub timeout: Duration,
    pub relay: Option<Relay>
}

impl PublicClient {
    // pub fn default() -> Self {
    //     PublicClient::local(3030)
    // }

    pub fn client_wrapper(&self) -> rest::RgHttpClient {
        rest::RgHttpClient::new(self.url.clone(), self.port as u16, self.relay.clone())
    }

    pub fn local(port: u16, _relay: Option<Relay>) -> Self {
        Self {
            url: "localhost".to_string(),
            port,
            timeout: Duration::from_secs(120),
            relay: None,
        }
    }

    pub fn from(url: String, port: u16, relay: Option<Relay>) -> Self {
        Self {
            url,
            port,
            timeout: Duration::from_secs(120),
            relay,
        }
    }


    fn formatted_url(&self) -> String {
        return "http://".to_owned() + &*self.url.clone() + ":" + &*self.port.to_string();
    }

    fn formatted_url_metrics(&self) -> String {
        return "http://".to_owned() + &*self.url.clone() + ":" + &*(self.port - 2).to_string();
    }

    #[allow(dead_code)]
    pub async fn request(&self, r: &PublicRequest) -> Result<PublicResponse, ErrorInfo> {
        use reqwest::ClientBuilder;
        use redgold_schema::error_info;
        let client = ClientBuilder::new().timeout(self.timeout).build().unwrap();
        // // .default_headers(headers)
        // // .gzip(true)
        // .timeout(self.timeout)
        // .build()?;
        // info!("{:?}", "");
        // info!(
        //     "Sending PublicRequest: {:?}",
        //     serde_json::to_string(&r.clone()).unwrap()
        // );
        let sent = client
            .post(self.formatted_url() + "/request")
            .json(r)
            .send();
        let response = sent.await;
        match response {
            Ok(r) => match r.json::<PublicResponse>().await {
                Ok(res) => Ok(res),
                Err(e) => Err(schema::error_info(e.to_string())),
            },
            Err(e) => Err(error_info(e.to_string())),
        }
    }


    pub async fn send_transaction(
        &self,
        t: &Transaction,
        sync: bool,
    ) -> Result<SubmitTransactionResponse, ErrorInfo> {

        let mut c = self.client_wrapper();
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


    pub async fn faucet(
        &self,
        t: &Address
    ) -> Result<FaucetResponse, ErrorInfo> {
        let mut request = Request::default();
        request.faucet_request = Some(FaucetRequest {
            address: Some(t.clone()),
            token: None
        });
        info!("Sending faucet request: {}", t.clone().render_string().expect("r"));
        let response = self.client_wrapper().proto_post_request(request, None, None).await?;
        let fr = response.faucet_response.safe_get()?;
        let res = json(&response)?;
        info!("Faucet response: {}", res);
        Ok(fr.clone())
    }

    #[allow(dead_code)]
    pub async fn query_address(
        &self,
        addresses: Vec<Address>,
    ) -> Result<PublicResponse, ErrorInfo> {
        let mut request = empty_public_request();
        request.query_addresses_request = Some(QueryAddressesRequest {
                addresses
            });
        self.request(&request).await
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
        Ok(self.client_wrapper().proto_post_request(request, None, None).await?.hash_search_response.safe_get()?.clone())
    }


    pub async fn about(&self) -> Result<AboutNodeResponse, ErrorInfo> {
        let mut request = empty_public_request();
        request.about_node_request = Some(AboutNodeRequest{ verbose: true });
        let result = self.request(&request).await;
        let result1 = result?.as_error();
        let option = result1?.about_node_response;
        let result2 = option.safe_get_msg("Missing response");
        Ok(result2?.clone())
    }

}