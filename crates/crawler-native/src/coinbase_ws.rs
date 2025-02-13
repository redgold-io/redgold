use futures_util::{SinkExt, StreamExt};
use metrics::counter;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::{ErrorInfoContext, RgResult};
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

async fn run_websocket_stream_inf(
    url: String, initial_subscribe_message: String, messages: flume::Sender<String>
) -> RgResult<()> {
    loop {
        counter!("redgold_ws_stream_init").increment(1);
        run_websocket_stream(url.clone(), initial_subscribe_message.clone(), messages.clone()).await.log_error().ok();
    }
}
async fn run_websocket_stream(url: String, initial_subscribe_message: String, messages: flume::Sender<String>) -> RgResult<()> {

    let url = Url::parse(&*url).error_info("Websocket url parse failure")?;

    let (ws_stream, _) = connect_async(url.to_string()).await.error_info("Websocket connect failure")?;

    let (mut write, mut read) = ws_stream.split();

    write.send(Message::Text(initial_subscribe_message.to_string())).await.error_info("Initial subscribe failure")?;
    // Handle incoming messages
    while let Some(message) = read.next().await {
        counter!("redgold_ws_stream_message").increment(1);
        let message = message.error_info("Websocket read failure")?;
        match message {
            Message::Text(text) => {
                messages.send(text).error_info("Message send failure")?;
            }
            Message::Close(..) => {
                return "WebSocket closed".to_error();
            }
            _ => {}
        }
    }
    Ok(())

}

pub async fn run_coinbase_ws_status(messages: flume::Sender<String>) -> RgResult<()> {

    let url = "wss://ws-feed.exchange.coinbase.com".to_string();
    // Subscribe to the status channel
    let subscribe_message = json!({
        "type": "subscribe",
        "channels": [{ "name": "status" }]
    });

    run_websocket_stream_inf(
        url,
        subscribe_message.to_string(),
        messages
    ).await
}


#[tokio::test]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let url = Url::parse("wss://ws-feed.exchange.coinbase.com")?;

    let (ws_stream, _) = connect_async(url.to_string()).await?;
    println!("WebSocket handshake has been successfully completed");

    let (mut write, mut read) = ws_stream.split();

    // Subscribe to the status channel
    let subscribe_message = json!({
        "type": "subscribe",
        "channels": [{ "name": "status" }]
    });

    write.send(Message::Text(subscribe_message.to_string())).await?;
    println!("Subscribed to status channel");

    // Handle incoming messages
    while let Some(message) = read.next().await {
        match message {
            Ok(Message::Text(text)) => {
                println!("Received: {}", text);
                // You can parse and process the JSON message here
                break;
            }
            Ok(Message::Close(..)) => {
                println!("WebSocket closed");
                break;
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
            _ => {}
        }
    }

    Ok(())
}