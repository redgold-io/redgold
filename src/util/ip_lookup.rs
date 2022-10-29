
use reqwest::ClientBuilder;
use redgold_schema::{error_info, error_message};
use crate::schema::structs::ErrorInfo;

pub async fn get_self_ip() -> Result<String, ErrorInfo> {
    let client = ClientBuilder::new().build().expect("client build error");
    let res = client.get("https://api.ipify.org").send().await
        .map_err(|e| error_info(format!("IP lookup failure {}", e.to_string())))?;
    let text = res.text().await;
    text.map_err(|e| error_info(format!("IP lookup response text failure {}", e.to_string())))
}

// https://api.ipify.org

#[tokio::test]
async fn check_ip_lookup() {
    println!("{}", get_self_ip().await.expect("asdf"))
}
