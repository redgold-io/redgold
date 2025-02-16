use bitcoin::consensus::encode;
use bitcoin::Transaction;
use bitcoincore_rpc::{Auth, Client};
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::{ErrorInfoContext, RgResult};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
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
    pub async fn run(self) -> RgResult<()> {
        let tracker = self.confirmation_tracker.clone();
    
        // Setup ZMQ context and socket in the spawned thread
        let context = zmq::Context::new();
        let socket = context.socket(zmq::SUB).error_info("Bad socket")?;
    
        let tx_addr = format!("tcp://{}:{}", self.config.host, self.config.zmq_tx_port);
        let block_addr = format!("tcp://{}:{}", self.config.host, self.config.zmq_block_port);
        
        info!("Connecting to BTC ZMQ endpoints: {} and {}", tx_addr, block_addr);
        
        // Connect with retry logic
        let mut retry_count = 0;
        let max_retries = 5;
        
        while retry_count < max_retries {
            match (socket.connect(&tx_addr), socket.connect(&block_addr)) {
                (Ok(_), Ok(_)) => {
                    info!("Successfully connected to ZMQ endpoints");
                    break;
                }
                (Err(e1), Err(e2)) => {
                    error!("Failed to connect to ZMQ ports: {} and {}", e1, e2);
                    retry_count += 1;
                    if retry_count == max_retries {
                        error!("Max retries reached, giving up");
                        return "Max retries".to_error();
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    continue;
                }
                (Err(e), _) | (_, Err(e)) => {
                    error!("Failed to connect to one ZMQ port: {}", e);
                    retry_count += 1;
                    if retry_count == max_retries {
                        error!("Max retries reached, giving up");
                        return "Max retries".to_error();
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    continue;
                }
            }
    }
    
        // Subscribe to both transaction and block topics
        if let Err(e) = socket.set_subscribe(b"rawtx") {
            error!("Failed to subscribe to transactions: {}", e);
            return "Failed to subscribe to transactions".to_error();
        }
        if let Err(e) = socket.set_subscribe(b"rawblock") {
            error!("Failed to subscribe to blocks: {}", e);
            return "Failed to subscribe to blocks".to_error();
        }

        // Setup RPC client
        let rpc_url = format!("http://{}:{}", self.config.host, self.config.rpc_port);
        let _rpc_client = match Client::new(&rpc_url, Auth::None) {
            Ok(client) => client,
            Err(e) => {
                error!("Failed to create RPC client: {}", e);
                return "Failed to create RPC client".to_error();
            }
        };
        
       loop {
            let topic = match socket.recv_bytes(0) {
                Ok(t) => t,
                Err(e) => {
                    error!("Error receiving topic: {}", e);
                    continue;
                }
            };
            let content = match socket.recv_bytes(0) {
                Ok(c) => c,
                Err(e) => {
                    error!("Error receiving content: {}", e);
                    continue;
                }
            };

            match topic.as_slice() {
                b"rawtx" => {
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

                        // // Decode and process transaction
                        // for (i, output) in tx.output.iter().enumerate() {
                        //     info!("Output {}: {} satoshis", i, output.value);
                        //     if let Ok(address) = bitcoin::Address::from_script(&output.script_pubkey, bitcoin::Network::Bitcoin) {
                        //         info!("  Address: {}", address);
                        //     }
                        // }
                    }
                }
                b"rawblock" => {
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
                _ => {}
            }
        }
        Ok(())
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
        
        // // Run the subscriber in the background
        let handle = tokio::spawn(subscriber_clone.run());

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
        handle.abort();
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
