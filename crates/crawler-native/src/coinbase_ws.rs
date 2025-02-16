use std::collections::{HashMap, VecDeque};
use std::path::Path;

use futures_util::{FutureExt, SinkExt, Stream, StreamExt, TryStreamExt};
use metrics::counter;
use redgold_schema::errors::helpers::WithMetrics;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_crawler::coinbase::message_access::{MessageAccess};
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::observability::errors::Loggable;
use redgold_schema::{message, ErrorInfoContext, RgResult};
use serde_json::json;
use tokio::io::Join;
use tokio::task::JoinHandle;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;
use redgold_common_no_wasm::arc_swap_wrapper::WriteOneReadAll;
use redgold_crawler::coinbase::ticker_schema::TickerMessage;
use redgold_schema::structs::{ErrorInfo, SupportedCurrency};
use crate::coinbase_ws;

const MAX_TICKER_HISTORY: usize = 1000;

async fn run_websocket_stream_inf(
    url: String, initial_subscribe_message: String, messages: flume::Sender<String>, ws_identifier: impl Into<String>
) -> RgResult<()> {
    let suffix = ws_identifier.into();
    loop {
        counter!("redgold_ws_stream_init").increment(1);
        counter!(format!("redgold_ws_stream_init_{suffix}")).increment(1);
        run_websocket_stream(url.clone(), initial_subscribe_message.clone(), messages.clone(), &suffix).await
        .with_err_count(format!("redgold_ws_stream_err_{suffix}"))
        .log_error()
        .ok();
    }
}
async fn run_websocket_stream(url: String, initial_subscribe_message: String, messages: flume::Sender<String>, ws_identifier: impl Into<String>) -> RgResult<()> {

    let url = Url::parse(&*url).error_info("Websocket url parse failure")?;

    let (ws_stream, _) = connect_async(url.to_string()).await.error_info("Websocket connect failure")?;

    let (mut write, mut read) = ws_stream.split();

    write.send(Message::Text(initial_subscribe_message.to_string())).await.error_info("Initial subscribe failure")?;
    let suffix = ws_identifier.into();
    // Handle incoming messages
    while let Some(message) = read.next().await {
        counter!(format!("redgold_ws_stream_message_{suffix}")).increment(1);
        let message = message.error_info("Websocket read failure")?;
        match message {
            Message::Text(text) => {
                messages.send(text).error_info("Message send failure")
                .with_err_count(format!("redgold_ws_stream_message_err_{suffix}"))
                .log_error()
                .ok();
            }
            Message::Close(..) => {
                return "WebSocket closed".to_error();
            }
            _ => {}
        }
    }
    Ok(())

}

pub async fn run_coinbase_ws_ticker_batch(
    messages: flume::Sender<String>,
    product_ids: Option<Vec<SupportedCurrency>>
) -> RgResult<()> {

    let product_ids = product_ids.unwrap_or(vec![
        SupportedCurrency::Bitcoin,
        SupportedCurrency::Ethereum,
        SupportedCurrency::Solana,
        // SupportedCurrency::Monero
    ]);

    let url = "wss://ws-feed.exchange.coinbase.com".to_string();

    let product_ids = product_ids.into_iter().map(|p| p.quote_pair_usd()).collect::<Vec<_>>();

    let subscribe_message = json!({
        "type": "subscribe",
        "channels": ["ticker_batch"],
        "product_ids": product_ids
    });

    run_websocket_stream_inf(
        url,
        subscribe_message.to_string(),
        messages,
        "coinbase_"
    ).await
}


#[derive(Clone, Default)]
pub struct CoinbaseWsTicker {
    pub latest_messages_all: VecDeque<TickerMessage>,
    pub latest_price: HashMap<SupportedCurrency, f64>,
    pub latest_by: HashMap<SupportedCurrency, TickerMessage>
}

impl CoinbaseWsTicker {
    fn push_message(&mut self, message: TickerMessage) {
        while self.latest_messages_all.len() >= MAX_TICKER_HISTORY {
            self.latest_messages_all.pop_front();
        }
        self.latest_messages_all.push_back(message);
    }
}

pub struct CoinbaseWsTickerStart {
    pub ws_thread: JoinHandle<RgResult<()>>,
    pub decoder_thread: JoinHandle<RgResult<()>>,
    pub ticker: WriteOneReadAll<CoinbaseWsTicker>
}

impl CoinbaseWsTickerStart {
    pub fn abort(&self) {
        self.ws_thread.abort();
        self.decoder_thread.abort();
    }
}


pub async fn run_decoded_coinbase_ws(ticker: WriteOneReadAll<CoinbaseWsTicker>) -> CoinbaseWsTickerStart {
    let (s,r) = flume::bounded(100_000);
    let runner = tokio::spawn(run_coinbase_ws_ticker_batch(s, None));
    let t2 = ticker.clone();
    let decoder = tokio::spawn(async move {
        let t2 = t2;
        r.into_stream().map(|x| Ok::<String, ErrorInfo>(x))
        .try_fold(t2, |mut t2, x| async move {
            if let Ok(msg) = x.json_from::<redgold_crawler::coinbase::ticker_schema::Message>()
            .with_err_count("coinbase_ws_ticker_decode_err") {
                match msg {
                    redgold_crawler::coinbase::ticker_schema::Message::Ticker(ticker_message) => {
                        let p = ticker_message.price.parse::<f64>().unwrap();
                        let pid = ticker_message.product_id.replace("-USD", "");
                        if let Ok(currency) = SupportedCurrency::try_from(pid) {
                            let mut data = t2.clone_read();
                            data.push_message(ticker_message.clone());
                            data.latest_by.insert(currency, ticker_message.clone());
                            data.latest_price.insert(currency, p);
                            t2.write(data);
                        }
                    }
                    _ => {

                    }
                }
            }
            Ok(t2)
        }).await.ok();
        Ok(())
    });
    
    CoinbaseWsTickerStart {
        ws_thread: runner,
        decoder_thread: decoder,
        ticker
    }
}


#[tokio::test]
pub async fn test_coinbase_ws() {
    let (s,r) = flume::unbounded();
    let _runner = tokio::spawn(run_coinbase_ws_ticker_batch(s, None));

    // Give some time for the connection to establish
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let mut count = 0;
    let timeout = tokio::time::Duration::from_secs(5);
    let mut messages = Vec::new();
    let max_messages = 2;

    while count < max_messages {
        match tokio::time::timeout(timeout, r.recv_async()).await {
            Ok(Ok(message)) => {
                println!("Received: {}", message);
                count += 1;
                messages.push(message);
            }
            Ok(Err(e)) => {
                println!("Error receiving message: {}", e);
                break;
            }
            Err(_) => {
                println!("Timeout waiting for message");
                break;
            }
        }
    }


    let output = Path::new("coinbase_ws_output.txt");
    std::fs::remove_file(output).unwrap_or_default();
    std::fs::File::create(output).unwrap();
    std::fs::write(output, messages.join("\n")).unwrap();

    assert_eq!(messages.len(), max_messages);

    for m in messages {
        println!("Message: {}", m);
        let msg = m.json_from::<redgold_crawler::coinbase::ticker_schema::Message>().unwrap();
        match msg {
            redgold_crawler::coinbase::ticker_schema::Message::Ticker(ticker_message) => {
                let p = ticker_message.price.parse::<f64>().unwrap();
                println!("Price: {}", p);
                assert!(p > 10.0);
            }
            _ => {

            }
    
        }
    }

    _runner.abort();
}

// Usage example below copied from coinbase docs

#[ignore]
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
                // break;
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
