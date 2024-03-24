use async_trait::async_trait;
use log::info;
use metrics::counter;
use redgold_schema::{EasyJson, RgResult, SafeOption, WithMetadataHashable};
use redgold_schema::errors::EnhanceErrorInfo;
use redgold_schema::structs::{ErrorInfo, Transaction};
use crate::core::internal_message::SendErrorInfo;
use crate::core::relay::Relay;
use crate::core::stream_handlers::TryRecvForEach;
use crate::observability::logging::Loggable;
use crate::observability::metrics_help::WithMetrics;
use crate::util;


#[derive(Clone)]
pub struct TransactionWithSender {
    pub transaction: Transaction,
    pub sender: flume::Sender<RgResult<()>>
}

#[derive(Clone)]
pub enum TxWriterMessage {
    WriteTransaction(TransactionWithSender)
}

#[derive(Clone)]
pub struct TxWriter {
    relay: Relay
}

impl TxWriter {
    pub fn new(relay: &Relay) -> Self {
        Self {
            relay: relay.clone()
        }
    }

    pub async fn write_transaction(&self, transaction: &Transaction) -> RgResult<()> {

        info!("Writing transaction: {}", transaction.hash_or());
        counter!("redgold.transaction.tx_writer.write_transaction").increment(1);
        // Validate again immediately
        for utxo_id in transaction.utxo_inputs() {
            let valid = self.relay.ds.utxo.utxo_id_valid(utxo_id).await?;
            if !valid {
                return Err(ErrorInfo::new("Invalid UTXO")).add(utxo_id.json_or());
            }
            let child = self.relay.ds.utxo.utxo_children(utxo_id).await?;
            if child.len() > 0 {
                return Err(ErrorInfo::new("UTXO has children"))
                    .add(utxo_id.json_or())
                    .add(child.json_or());
            }
        }
        // TODO: Handle graceful rollback on failure here.
        for fixed in transaction.utxo_inputs() {
            // This should maybe just put the whole node into a corrupted state? Or a retry?
            self.relay.ds.utxo.delete_utxo(&fixed).await.mark_abort()?;
            let utxo_valid = self.relay.ds.transaction_store.query_utxo_id_valid(
                &fixed.transaction_hash.safe_get()?.clone(), fixed.output_index
            ).await.mark_abort()?;
            let deleted = !utxo_valid;
            if !deleted {
                return Err(ErrorInfo::new("UTXO not deleted")).add(fixed.json_or())
                    .mark_abort();
            }
        }

        // Do as a single commit with a rollback.
        // Preserve all old data / inputs while committing new transaction, or do retries?
        // or fail the entire node?
        // TODO: Should each UTXO key handler thread handle the deletion of the UTXO? Should we 'block' the utxo entry?
        // with a message? Or should we just delete it here?
        // Commit transaction internally to database.

        self.relay
                .ds
                .transaction_store
                .insert_transaction(
                    &transaction, util::current_time_millis_i64(), true, None, true
                ).await.mark_abort()?;
        info!("Wrote transaction: {}", transaction.hash_or());

        return Ok(());

    }
    pub async fn process_message(&mut self, message: TxWriterMessage) -> RgResult<()> {
        counter!("redgold.transaction.tx_writer.process_message").increment(1);
        match message {
            TxWriterMessage::WriteTransaction(tws) => {
                let result = self.write_transaction(&tws.transaction).await
                    .bubble_abort()?
                    .log_error()
                    .with_err_count("redgold.transaction.tx_writer.write_transaction.error");
                tws.sender.send_rg_err(result)
            }
        }
    }
}

#[async_trait]
impl TryRecvForEach<TxWriterMessage> for TxWriter {
    async fn try_recv_for_each(&mut self, message: TxWriterMessage) -> RgResult<()> {
        self.process_message(message).await
    }
}