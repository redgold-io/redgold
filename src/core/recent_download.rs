use async_trait::async_trait;
use itertools::Itertools;
use metrics::counter;
use redgold_schema::RgResult;
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::util::xor_distance::{xorf_conv_distance, xorfc_hash};
use redgold_common::flume_send_help::SendErrorInfo;
use crate::core::relay::Relay;
use crate::core::resolver::{resolve_input, ResolvedInput};
use crate::core::stream_handlers::IntervalFold;


enum DownloadTrigger {
    OnInputResolution(ResolvedInput),

}

pub struct RecentDownload {
    pub(crate) relay: Relay,
}

impl RecentDownload {
    pub async fn process_resolved_input(&self, update: ResolvedInput) -> RgResult<()> {
        let h = update.parent_transaction.hash_or();
        let within_distance = self.relay.tx_hash_distance(&h).await?;
        if within_distance {
            let mut store = false;
            // Determine whether or not we should download this transaction
            // and consider it accepted.

            for pk in update.observation_proofs.iter()
                .flat_map(|o| o.proof.as_ref()
                    .and_then(|p| p.public_key.as_ref())
                ) {
                let t = self.relay.get_security_rating_trust_of_node(&pk).await?;
                if let Some(t) = t {
                    // TODO: Better summation based on total peers and distance and so on
                    if t > 0.5 {
                        store = true;
                    }
                }
            }
            if store {
                counter!("redgold.recent_download.accepted_transactions").increment(1);
                self.relay.write_transaction(
                    &update.parent_transaction,
                    update.parent_transaction.time()?.clone(),
                    None,
                    true
                ).await?;
                //
                // for u in &update.parent_transaction.utxo_outputs().unwrap_or(vec![]) {
                //     if let Some(id) = &u.utxo_id {
                //         if self.relay.ds.transaction_store.utxo_used(id).await?.is_none() {
                //             let valid = self.relay.utxo_id_valid_peers(id).await?;
                //             // TODO: Check the conflict manager to see if this UTXO is under contention?
                //             if valid.is_none() {
                //                 self.relay.ds.transaction_store.insert_utxo(u, None).await?;
                //                 if !(self.relay.utxo_channels.contains_key(id)) {
                //                     self.relay.ds.transaction_store.insert_utxo(u, None).await?;
                //                 }
                //             }
                //         }
                //     }
                // }

                // TODO: Check all UTXOs and see if they are in range and known

                for input in &update.parent_transaction.inputs {
                    if let Some(h) = input.utxo_id.as_ref().and_then(|i| i.transaction_hash.as_ref()) {
                        if !self.relay.ds.transaction_store.transaction_known(h).await? {
                            let resolved_input = resolve_input(
                                input.clone(),
                                self.relay.clone(),
                                vec![],
                                // Not needed here
                                update.parent_transaction.signable_hash(),
                                false,
                                update.parent_transaction.time()?.clone(),
                            ).await
                                .with_detail("resolve_invocation", "recent_download");
                            if let Ok(resolved_input) = resolved_input {
                                self.relay.unknown_resolved_inputs.sender.send_rg_err(resolved_input)?;
                            } else {
                                counter!("redgold.recent_download.resolve_input_error").increment(1);
                            }
                        }
                    }

                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl IntervalFold for RecentDownload {
    async fn interval_fold(&mut self) -> RgResult<()> {
        let updates = self.relay.unknown_resolved_inputs.recv_while()?;
        for update in updates {
            self.process_resolved_input(update).await?;
        }
        Ok(())
    }
}