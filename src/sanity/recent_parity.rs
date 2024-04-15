use std::collections::HashSet;
use async_trait::async_trait;
use log::info;
use redgold_schema::{EasyJson, error_info, RgResult};
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::{Hash, Transaction, TransactionEntry, UtxoId};
use crate::core::relay::Relay;
use crate::core::stream_handlers::IntervalFold;
use crate::core::transact::tx_validate::TransactionValidator;
use crate::observability::send_email::EmailOnError;
use crate::util;

pub struct RecentParityCheck {
    relay: Relay
}

impl RecentParityCheck {
    pub fn new(relay: &Relay) -> Self {
        Self {
            relay: relay.clone()
        }
    }
    pub async fn run(&self) -> RgResult<()> {

        // TODO: The XOR scale version of this function requires peer queries to validate historical
        // since we expect not all within distance.
        let ct = util::current_time_millis_i64();
        // Last day of TX
        info!("Started recent sanity check for parity");
        let begin = ct - 1000 * 60 * 60 * 24;
        let recent_tx = self.relay.ds.transaction_store
            .query_time_transaction_accepted_ordered(begin, ct).await.mark_abort()?;
        self.validate_transaction_window(&recent_tx).await?;
        Ok(())
    }

    async fn validate_transaction_window(&self, recent_tx: &Vec<Transaction>) -> RgResult<()> {
        let mut used_utxo_ids: HashSet<UtxoId> = HashSet::new();
        let mut tx_hashes: HashSet<Hash> = HashSet::new();
        let fee_addrs = self.relay.default_fee_addrs();
        for tx in recent_tx {
            let hash = tx.hash_or();
            let tx_json = tx.json_or();
            tx.validate(Some(&fee_addrs), Some(&self.relay.node_config.network))
                .add("Transaction validation failure in recent parity check")
                .with_detail("tx", tx_json.clone())
                .with_detail("hash", hash.hex())?;

            if tx_hashes.contains(&hash) {
                return Err(error_info("Duplicate transaction hash in recent transactions"))
                    .with_detail("tx", tx_json)
                    .with_detail("hash", hash.hex());
            } else {
                tx_hashes.insert(hash);
            }
            // Assume all inputs are validated as of cut-off point.
            for utxo_id in tx.input_utxo_ids() {
                if used_utxo_ids.contains(utxo_id) {
                    return Err(error_info("Utxo Id in input already used in recent transactions"))
                        .with_detail("tx", tx_json)
                        .with_detail("utxo_id", utxo_id.json_or());
                }
                used_utxo_ids.insert(utxo_id.clone());
                let children = self.relay.ds.utxo.utxo_children(utxo_id).await.mark_abort()?;
                if children.len() > 1 {
                    return Err(error_info("Utxo Id has more than one child in recent transactions"))
                        .with_detail("tx", tx_json)
                        .with_detail("utxo_id", utxo_id.json_or())
                        .with_detail("children", children.json_or());
                }
            }
            for utxo_id in tx.output_utxo_ids() {
                if used_utxo_ids.contains(utxo_id) {
                    return Err(error_info("Utxo Id in output already used in recent transactions"))
                        .with_detail("tx", tx_json)
                        .with_detail("utxo_id", utxo_id.json_or());
                }
                // used_utxo_ids.insert(utxo_id.clone());
            }
            //e.utxo_outputs()
            // }
        }
        Ok(())
    }
}

#[async_trait]
impl IntervalFold for RecentParityCheck {
    async fn interval_fold(&mut self) -> RgResult<()> {
        self.run().await.add("RecentParityCheck failure")
            .email_on_error()
            .await
            .bubble_abort()?
    }
}