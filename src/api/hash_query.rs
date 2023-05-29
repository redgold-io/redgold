use redgold_schema::from_hex;
use redgold_schema::structs::{Address, AddressInfo, ErrorInfo, Hash, HashSearchResponse, Transaction, TransactionInfo};
use crate::core::relay::Relay;
use crate::data::data_store::DataStore;

pub async fn hash_query(relay: Relay, hash_input: String) -> Result<HashSearchResponse, ErrorInfo> {
    let mut response = HashSearchResponse {
        transaction_info: None,
        address_info: None,
        observation: None,
        peer_data: None,
    };
    if let Ok(a) = Address::parse(hash_input.clone()) {
        let res = DataStore::map_err_sqlx(relay.ds.query_utxo_address(vec![a.clone()]).await)?;
        response.address_info = Some(AddressInfo::from_utxo_entries(a, res));
        return Ok(response);
    } else {
        let h = from_hex(hash_input)?;
        let hash = Hash::new(h.clone());
        let maybe_transaction = relay.ds.transaction_store.query_maybe_transaction(&hash).await?;
        let mut observation_proofs = vec![];
        let mut transaction = None;
        let mut rejection_reason = None;
        if let Some((t, e)) = maybe_transaction.clone() {
            observation_proofs = relay.ds.observation.select_observation_edge(&hash.clone()).await?;
            transaction = Some(t);
            rejection_reason = e;
        }
        // Query UTXO by hash only for all valid outputs.
        let valid_utxo_output_ids = relay.ds.transaction_store
            .query_utxo_output_index(&hash)
            .await?;

        response.transaction_info = Some(TransactionInfo{
            transaction,
            observation_proofs,
            valid_utxo_index: valid_utxo_output_ids,
            used_outputs: vec![],
            accepted: rejection_reason.is_none(),
            rejection_reason,
            queried_output_index_valid: None,
        })
    }
    Ok(response)
}