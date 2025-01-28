use crate::core::relay::Relay;
use redgold_schema::structs::{ErrorInfo, Hash, UtxoId};

// TODO: Implement schema etc.
pub async fn check_utxo_conflicts(_relay: Relay, _utxo_ids: &Vec<UtxoId>, _hash: &Hash) -> Result<(), ErrorInfo> {
    Ok(())
}