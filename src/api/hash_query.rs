use redgold_schema::{error_info, from_hex};
use redgold_schema::structs::{Address, AddressInfo, ErrorInfo, Hash, HashSearchResponse, PeerId, PublicKey, Transaction, TransactionInfo, TransactionState};
use redgold_schema::util::btc_wallet::SingleKeyBitcoinWallet;
use crate::core::relay::Relay;
use crate::data::data_store::DataStore;

pub async fn hash_query(relay: Relay, hash_input: String, limit: Option<i64>, offset: Option<i64>) -> Result<HashSearchResponse, ErrorInfo> {
    let mut response = HashSearchResponse {
        transaction_info: None,
        address_info: None,
        observation: None,
        peer_node_info: None,
        peer_id_info: None
    };

    let mut addr = None;

    if let Ok(a) = SingleKeyBitcoinWallet::parse_address(&hash_input) {
        addr = Some(Address::from_bitcoin(&hash_input));
    } else if let Ok(a) = Address::parse(hash_input.clone()) {
        addr = Some(a);
    }
    if let Some(a) = addr {
        let res = relay.ds.transaction_store.query_utxo_address(&a).await?;
        let mut info = AddressInfo::from_utxo_entries(a.clone(), res);
        let limit = limit.unwrap_or(10);
        let offset = offset.unwrap_or(0);
        info.recent_transactions = relay.ds.transaction_store.get_all_tx_for_address(&a, limit, offset).await?;
        response.address_info = Some(info);
        return Ok(response);
    } else {
        let h = from_hex(hash_input.clone())?;
        let hash = Hash::new(h.clone());
        let maybe_transaction = relay.ds.transaction_store.query_maybe_transaction(&hash).await?;
        let mut observation_proofs = vec![];
        let mut transaction = None;
        let mut rejection_reason = None;
        let mut state = TransactionState::Pending;

        if let Some((t, e)) = maybe_transaction.clone() {
            observation_proofs = relay.ds.observation.select_observation_edge(&hash.clone()).await?;
            transaction = Some(t);
            rejection_reason = e;
            // Query UTXO by hash only for all valid outputs.
            let valid_utxo_output_ids = relay.ds.transaction_store
                .query_utxo_output_index(&hash)
                .await?;

            let accepted = rejection_reason.is_none();


            if accepted {
                state = TransactionState::Accepted;
            } else {
                state = TransactionState::Rejected
            }

            response.transaction_info = Some(TransactionInfo {
                transaction,
                observation_proofs,
                valid_utxo_index: valid_utxo_output_ids,
                used_outputs: vec![],
                accepted: accepted,
                rejection_reason,
                queried_output_index_valid: None,
                state: state as i32,
            });
            return Ok(response)
        }
    }

    if let Some(pk) = PublicKey::from_hex(hash_input.clone()).ok() {
        if relay.node_config.public_key() == pk {
            response.peer_node_info = Some(relay.peer_node_info().await?);
            return Ok(response);
        }
        if let Some(pni) = relay.ds.peer_store.query_public_key_node(pk).await? {
            response.peer_node_info = Some(pni);
            return Ok(response);
        }
    }

    let result = from_hex(hash_input.clone())?;
    let id = PeerId::from_bytes(result.clone());
    if relay.node_config.peer_id == id {
        response.peer_id_info = Some(relay.peer_id_info().await?);
        return Ok(response);
    }

    if let Some(pid_info) = relay.ds.peer_store
        .query_peer_id_info(&id)
        .await? {
        response.peer_id_info = Some(pid_info);
        return Ok(response);
    }

    if let Some(h) = Hash::from_hex(hash_input.clone()).ok() {
        let r = relay.ds.observation.query_observation(&h).await?;
        if let Some(r) = r {
            response.observation = r.observation.clone();
            return Ok(response);
        }
    }

    // Err(error_info("Hash not found"))
    Ok(response)
}