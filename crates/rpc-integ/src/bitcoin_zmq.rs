use bitcoin::consensus::encode;
use bitcoin::Transaction;
use bitcoincore_rpc::{Auth, Client};
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::{ErrorInfoContext, RgResult};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{info, error};
use flume::{Sender, Receiver};

#[derive(Debug)]
pub enum ZmqMessage {
    Transaction(String),  // txid
    Block,
}

#[derive(Debug, Clone)]
pub struct BitcoinZmqConfig {
    pub host: String,
    pub zmq_tx_port: u16,
    pub zmq_block_port: u16,
    pub rpc_port: u16,
}

impl Default for BitcoinZmqConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            zmq_tx_port: 28332,
            zmq_block_port: 28333,
            rpc_port: 8332,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BitcoinZmqSubscriber {
    confirmation_tracker: Arc<Mutex<HashMap<String, u32>>>,
    message_sender: Sender<ZmqMessage>,
    message_receiver: Receiver<ZmqMessage>,
    config: BitcoinZmqConfig,
}

impl BitcoinZmqSubscriber {
    pub async fn new(config: Option<BitcoinZmqConfig>) -> Self {
        let (sender, receiver) = flume::unbounded();
        let s = Self {
            confirmation_tracker: Arc::new(Mutex::new(HashMap::new())),
            message_sender: sender,
            config: config.unwrap_or_default(),
            message_receiver: receiver,
        };
        s        
    }
    pub fn run(self) -> RgResult<()> {
        let tracker = self.confirmation_tracker.clone();
        
        let context = tmq::Context::new();
        let mut socket = context.socket(zmq::SUB).expect("Failed to create socket");
        
        let tx_addr = format!("tcp://{}:{}", self.config.host, self.config.zmq_tx_port);
        let block_addr = format!("tcp://{}:{}", self.config.host, self.config.zmq_block_port);
        
        info!("Connecting to BTC ZMQ endpoints: {} and {}", tx_addr, block_addr);
        
        // Connect to endpoints
        socket.connect(&tx_addr).expect("Failed to connect to tx endpoint");
        socket.connect(&block_addr).expect("Failed to connect to block endpoint");
        
        // Set subscriptions
        socket.set_subscribe(b"rawtx").expect("Failed to subscribe to rawtx");
        socket.set_subscribe(b"rawblock").expect("Failed to subscribe to rawblock");
        
        // Setup RPC client
        let rpc_url = format!("http://{}:{}", self.config.host, self.config.rpc_port);
        let _rpc_client = Client::new(&rpc_url, Auth::None)
            .error_info("Failed to create RPC client")?;
            
        loop {

            // Use tmq's async recv
            let topic = socket.recv_bytes(0).expect("Error receiving topic");
            let content = socket.recv_bytes(0).unwrap_or_default();
            
            if topic.starts_with(b"rawtx") {
                if let Ok(tx) = encode::deserialize::<Transaction>(&content) {
                    let txid = tx.compute_txid().to_string();
                    info!("Received new transaction: {}", txid);
                    
                    // Store initial confirmation count
                    if let Ok(mut tracker) = tracker.lock() {
                        tracker.insert(txid.clone(), 0);
                    }
                    
                    // Send message through channel
                    if let Err(e) = self.message_sender.send(ZmqMessage::Transaction(txid.clone())) {
                        error!("Failed to send transaction message: {}", e);
                    }
                }
            } else if topic.starts_with(b"rawblock") {
                // When we receive a new block, increment confirmation count for all tracked transactions
                if let Ok(mut tracker) = tracker.lock() {
                    for count in tracker.values_mut() {
                        *count += 1;
                    }
                    
                    // Log confirmation levels
                    for (txid, confirmations) in tracker.iter() {
                        info!("Transaction {} has {} confirmations", txid, confirmations);
                    }
                    
                    // Send block message
                    if let Err(e) = self.message_sender.send(ZmqMessage::Block) {
                        error!("Failed to send block message: {}", e);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use redgold_common::log::init_logger_once;
    use tokio::time::sleep;

    // Helper function to create a test configuration
    fn create_test_config() -> BitcoinZmqConfig {
        BitcoinZmqConfig {
            host: "server".to_string(),
            zmq_tx_port: 28332,
            zmq_block_port: 28333,
            rpc_port: 8332,
        }
    }

    #[tokio::test]
    async fn test_bitcoin_zmq_subscriber() {
        init_logger_once();
        let config = create_test_config();
        let subscriber = BitcoinZmqSubscriber::new(Some(config)).await;
        let subscriber_clone = subscriber.clone();
        info!("Running subscriber");
        
        let handle = std::thread::spawn(move || {
            subscriber_clone.run();
        });

        for i in 0..5 {
            info!("Waiting for message {}", i);
            let res = tokio::time::timeout(
                Duration::from_secs(2), 
                subscriber.message_receiver.recv_async()
            ).await;
            match res {
                Ok(msg) => {
                    info!("Received message: {:?}", msg);
                }
                Err(e) => {
                    info!("Timeout waiting for message {}", i);
                }
            }
        }
        info!("Subscriber aborted");  

        // handle.abort();
    }

    #[tokio::test]
    async fn test_bitcoin_zmq_step_by_step() {
        let config = create_test_config();
        
        // Step 1: Create ZMQ context and socket
        println!("Step 1: Creating ZMQ context and socket");
        let context = zmq::Context::new();
        let socket = match context.socket(zmq::SUB) {
            Ok(s) => {
                println!("Successfully created ZMQ socket");
                s
            },
            Err(e) => {
                error!("Failed to create ZMQ socket: {}", e);
                panic!("Socket creation failed");
            }
        };

        // Step 2: Connect to transaction endpoint
        let tx_addr = format!("tcp://{}:{}", config.host, config.zmq_tx_port);
        println!("Step 2: Connecting to transaction endpoint: {}", tx_addr);
        match socket.connect(&tx_addr) {
            Ok(_) => println!("Successfully connected to transaction endpoint"),
            Err(e) => {
                error!("Failed to connect to transaction endpoint: {}", e);
                panic!("Transaction endpoint connection failed");
            }
        }

        // Step 3: Connect to block endpoint
        let block_addr = format!("tcp://{}:{}", config.host, config.zmq_block_port);
        println!("Step 3: Connecting to block endpoint: {}", block_addr);
        match socket.connect(&block_addr) {
            Ok(_) => println!("Successfully connected to block endpoint"),
            Err(e) => {
                error!("Failed to connect to block endpoint: {}", e);
                panic!("Block endpoint connection failed");
            }
        }

        // Step 4: Subscribe to transaction and block topics
        println!("Step 4: Setting up subscriptions");
        if let Err(e) = socket.set_subscribe(b"rawtx") {
            error!("Failed to subscribe to transactions: {}", e);
            panic!("Transaction subscription failed");
        }
        if let Err(e) = socket.set_subscribe(b"rawblock") {
            error!("Failed to subscribe to blocks: {}", e);
            panic!("Block subscription failed");
        }
        println!("Successfully subscribed to both topics");

        // Step 5: Try to receive messages
        println!("Step 5: Attempting to receive messages");
        
        // Set a timeout for the socket
        socket.set_rcvtimeo(5000).expect("Failed to set receive timeout");
        
        // Try to receive a message
        match socket.recv_bytes(0) {
            Ok(topic) => {
                println!("Received topic: {:?}", String::from_utf8_lossy(&topic));
                match socket.recv_bytes(0) {
                    Ok(content) => {
                        println!("Received content of length: {}", content.len());
                        if topic == b"rawtx" {
                            if let Ok(tx) = encode::deserialize::<Transaction>(&content) {
                                let txid = tx.compute_txid().to_string();
                                println!("Successfully decoded transaction: {}", txid);
                            } else {
                                error!("Failed to decode transaction");
                            }
                        }
                    }
                    Err(e) => error!("Failed to receive content: {}", e),
                }
            }
            Err(e) => {
                if e == zmq::Error::EAGAIN {
                    println!("No messages received within timeout - this is expected if bitcoind is not running");
                } else {
                    error!("Failed to receive topic: {}", e);
                }
            }
        }
    }
}
