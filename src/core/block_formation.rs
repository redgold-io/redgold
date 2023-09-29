use crate::core::relay::Relay;
use redgold_data::data_store::DataStore;
use crate::genesis::create_genesis_block;
use crate::schema::error_message;
use crate::schema::structs::{Address, AddressBlock, Block, Error, ErrorInfo, Output};
use crate::schema::WithMetadataHashable;
use crate::schema::{struct_metadata, SafeBytesAccess};
use crate::util;
use crate::util::rg_merkle;
use itertools::Itertools;
use log::{debug, error};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// This is only used for backwards compatibility purposes and should not be considered
/// the source of truth or irreversibility.
pub struct BlockFormationProcess {
    relay: Relay,
    last_block: Block,
}

/// This is only used for backwards compatibility purposes and should not be considered
/// the source of truth or irreversibility.
impl BlockFormationProcess {
    pub async fn run(mut self) -> Result<(), ErrorInfo> {
        let mut interval = tokio::time::interval(self.relay.node_config.block_formation_interval);
        // TODO: everything in here is basically a placeholder for later
        loop {
            interval.tick().await;
            let i = self.last_block.time().expect("Last block time missing");
            let txs = self
                .relay
                .ds
                // TODO: No
                .transaction_store
                .query_time_transaction(i as i64, util::current_time_millis_i64()).await
                // .map_err(|e| error!("Error query_time_transaction {}", e.to_string()))
                .ok();
            if let Some(txa) = txs {
                // TODO: re-query observation edge here.
                let last_time = txa.iter().map(|t| t.time.clone()).max().expect("max");
                let vec = txa
                    .clone()
                    .iter()
                    .map(|e| e.transaction.as_ref().expect("").clone())
                    .collect_vec();
                let leafs = vec
                    .clone()
                    .iter()
                    .map(|e| e.hash_bytes().expect("vec").clone())
                    .collect_vec();
                let height = self.last_block.height + 1;
                let block = Block {
                    // TODO: use a real merkle root here
                    merkle_root: Some(rg_merkle::build_root_simple(&leafs)),
                    transactions: vec.clone(), // TODO: can leave this blank and enforce it properly
                    // to remove the clone on hash calculation? That's clever do it as part
                    // of a constructor function.
                    struct_metadata: struct_metadata(last_time.clone() as i64),
                    previous_block_hash: Some(self.last_block.hash_or()),
                    metadata: None,
                    height,
                }
                .with_hash()
                .clone();

                // todo: based on node config
                // TODO: Re-enable
                // self.relay.ds.insert_block_update_historicals(&block).await?;
                self.last_block = block.clone();
                metrics::increment_counter!("redgold.blocks.created");
                debug!("Formed block with hash {}", block.hash_hex_or_missing())
            }
        }
    }

    pub async fn default(relay: Relay) -> Result<Self, ErrorInfo> {
        let last_block_opt = relay
            .ds
            .address_block_store
            .query_last_block()
            .await?;
            // .expect("Datastore failure on last block query")
            // .ok_or(error_message(Error::UnknownError, "Missing last block"))
        //
        let last_block = match last_block_opt {
            None => {
                let block = create_genesis_block();
                // TODO: only if historical balance tracking enabled
                // relay.ds.insert_block_update_historicals(&block).await?;
                block
            }
            Some(b) => {b}
        };
        Ok(Self { relay, last_block })
    }
}

#[test]
fn test_block_formation() {}
