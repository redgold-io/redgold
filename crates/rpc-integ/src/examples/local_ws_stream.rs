use ethers::middleware::Middleware;
use ethers::prelude::StreamExt;
use ethers::providers::{Provider, Ws};

#[ignore]
#[tokio::test]
pub async fn ws_stream_test() {
    let provider = Provider::<Ws>::connect("ws://server:8556").await.expect("ws connect failed");

    // Subscribe to new blocks
    let p1 = provider.clone();
    let mut block_stream = p1.subscribe_blocks().await.expect("block subscription failed");
    // // Subscribe to pending transactions
    // let mut tx_stream = provider
    //     .subscribe_pending_txs()
    //     .await
    //     .expect("transaction subscription failed");

    println!("Subscribed to new blocks and pending transactions");

    // Spawn a task to handle block stream
    let block_provider = provider.clone();
    while let Some(block) = block_stream.next().await {
        println!("New block number: {:?}", block.number.unwrap());
        println!("Block hash: {:?}", block.hash.unwrap());
        println!("Parent hash: {:?}", block.parent_hash);
        println!("Gas used: {:?}", block.gas_used);

        // Get full block with transactions
        if let Ok(full_block) = block_provider
            .get_block_with_txs(block.number.unwrap())
            .await
        {
            if let Some(block_with_txs) = full_block {
                println!("Number of transactions: {}", block_with_txs.transactions.len());

                // Process each transaction in the block
                for tx in block_with_txs.transactions {
                    println!("Transaction hash: {:?}", tx.hash);
                    println!("From: {:?}", tx.from);
                    println!("To: {:?}", tx.to);
                    println!("Value: {:?}", tx.value);
                }
            }
        }
        println!("----------------------------------------");
    }
    //
    //
    // // Spawn a task to handle transaction stream
    // let tx_provider = provider.clone();
    // let tx_handle = tokio::spawn(async move {
    //     while let Some(tx_hash) = tx_stream.next().await {
    //         // Get full transaction details
    //         if let Ok(Some(tx)) = tx_provider.get_transaction(tx_hash).await {
    //             println!("Pending transaction detected:");
    //             println!("Hash: {:?}", tx.hash);
    //             println!("From: {:?}", tx.from);
    //             println!("To: {:?}", tx.to);
    //             println!("Value: {:?}", tx.value);
    //             println!("----------------------------------------");
    //         }
    //     }
    // });
    //
    // // Wait for both streams (you might want to add proper termination condition in production)
    // tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    // let _ = block_handle.await;
    // let _ = tx_handle.await;
}