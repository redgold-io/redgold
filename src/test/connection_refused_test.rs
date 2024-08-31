use redgold_schema::structs;
use crate::api::RgHttpClient;
use crate::core::relay::Relay;

#[ignore]
#[tokio::test]
async fn debug_conn_refused() {
    let r = Relay::dev_default().await;
    let c = RgHttpClient::new("n7.redgold.io".to_string(), 16481, None);
    let req = structs::Request::default();
    let response = c.proto_post_request(req, None, None).await;
    response.unwrap();
}