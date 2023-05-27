use redgold_schema::structs::{ErrorInfo, FixedUtxoId, Hash};
use crate::core::relay::Relay;

// TODO: Implement schema etc.
pub async fn check_utxo_conflicts(_relay: Relay, _utxo_ids: &Vec<FixedUtxoId>, _hash: &Hash) -> Result<(), ErrorInfo> {
    Ok(())
}