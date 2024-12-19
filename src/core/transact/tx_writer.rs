use async_trait::async_trait;
use tracing::info;
use metrics::counter;
use redgold_schema::{RgResult, SafeOption};
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::{ErrorInfo, Transaction};
use redgold_common::flume_send_help::SendErrorInfo;
use crate::core::relay::Relay;
use crate::core::stream_handlers::TryRecvForEach;
use redgold_schema::observability::errors::Loggable;
use crate::observability::metrics_help::WithMetrics;
use crate::util;


#[derive(Clone)]
pub struct TransactionWithSender {
    pub transaction: Transaction,
    pub sender: flume::Sender<RgResult<()>>,
    pub time: i64,
    pub rejection_reason: Option<ErrorInfo>,
    pub update_utxo: bool
}

#[derive(Clone)]
pub enum TxWriterMessage {
    WriteTransaction(TransactionWithSender),
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

    pub async fn write_transaction(&self, message: &TransactionWithSender) -> RgResult<()> {

        let transaction = &message.transaction;
        // info!("Writing transaction: {}", transaction.hash_or());
        counter!("redgold.transaction.tx_writer.write_transaction").increment(1);
        // Validate again immediately
        // for utxo_id in transaction.utxo_inputs() {
        //     let valid = self.relay.ds.utxo.utxo_id_valid(utxo_id).await?;
        //     if !valid {
        //         return Err(ErrorInfo::new("Invalid UTXO")).add(utxo_id.json_or());
        //     }
        //     let child = self.relay.ds.utxo.utxo_children(utxo_id).await?;
        //     if child.len() > 0 {
        //         return Err(ErrorInfo::new("UTXO has children"))
        //             .add(utxo_id.json_or())
        //             .add(child.json_or());
        //     }
        // }

        // TODO: Should each UTXO key handler thread handle the deletion of the UTXO? Should we 'block' the utxo entry?
        // with a message? Or should we just delete it here?
        // Commit transaction internally to database.

        // info!("Accepting transaction: {}", transaction.hash_or());
        let time = if message.rejection_reason.is_none() {
            transaction.time()?.clone()
        } else {
            message.time
        };
        self.relay
            .ds
            .accept_transaction(
                &transaction, time, message.rejection_reason.clone(), message.update_utxo
            ).await.log_error().add("Transaction writer internal failure").mark_abort()?;

        // info!("Sanity check on transaction: {}", transaction.hash_or());
        // Additional sanity check here
        for fixed in transaction.utxo_inputs() {
            if message.rejection_reason.is_none() && message.update_utxo {
                let utxo_valid = self.relay.ds.utxo.utxo_id_valid(fixed).await.mark_abort()?;
                let deleted = !utxo_valid;
                if !deleted {
                    return Err(ErrorInfo::new("UTXO not deleted")).add(fixed.json_or())
                        .mark_abort();
                }
            }

        }

        // info!("Wrote transaction: {}", transaction.hash_or());



        return Ok(());

    }
    pub async fn process_message(&mut self, message: TxWriterMessage) -> RgResult<()> {
        counter!("redgold.transaction.tx_writer.process_message").increment(1);
        match message {
            TxWriterMessage::WriteTransaction(tws) => {
                let result = self.write_transaction(&tws).await
                    .log_error()
                    .bubble_abort()?
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