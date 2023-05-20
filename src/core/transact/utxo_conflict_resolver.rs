use redgold_schema::structs::{ErrorInfo, FixedUtxoId, Hash};
use crate::core::relay::Relay;

// TODO: Implement schema etc.
pub async fn check_utxo_conflicts(relay: Relay, utxo_ids: &Vec<FixedUtxoId>, hash: &Hash) -> Result<(), ErrorInfo> {
    Ok(())
}