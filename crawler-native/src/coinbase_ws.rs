use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{SinkExt, StreamExt};
use url::Url;
use serde_json::json;

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