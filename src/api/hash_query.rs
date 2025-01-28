use crate::core::relay::Relay;
use redgold_keys::address_external::{ToBitcoinAddress, ToEthereumAddress};
use redgold_keys::address_support::AddressSupport;
use redgold_keys::proof_support::PublicKeySupport;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{Address, AddressInfo, ErrorInfo, Hash, HashSearchResponse, PeerId, PublicKey};

pub async fn hash_query(relay: Relay, hash_input: String, limit: Option<i64>, offset: Option<i64>) -> Result<HashSearchResponse, ErrorInfo> {
    let mut response = HashSearchResponse::default();


    // First check address
    // TODO: Move to unified parser function.
    if let Ok(a) = hash_input.parse_address().or(Address::raw_from_hex(hash_input.clone())) {
        let info = get_address_info(&relay, limit, offset, &a).await?;
        response.address_info = Some(info);
        return Ok(response);
    }

    // Then check transaction
    let fallback = Hash::from_raw_hex_transaction(hash_input.clone()).ok();
    let normal_proto_hex_hash = Hash::from_hex(hash_input.clone()).ok();
    if let Some(hash) = normal_proto_hex_hash.or(fallback) {
        let maybe_tx_info = relay.ds.resolve_transaction_hash(&hash).await?;
        if let Some(tx_info) = maybe_tx_info {
            response.transaction_info = Some(tx_info);
            return Ok(response)
        }
        let r = relay.ds.observation.query_observation(&hash).await?;
        if let Some(r) = r {
            response.observation = Some(r.clone());
            return Ok(response);
        }
    }


    if let Some(pk) = PublicKey::from_hex(hash_input.clone()).ok().or(
        PublicKey::from_hex_direct(hash_input.clone()).and_then(|a| a.validate().map(|_| a.clone())).ok()
    ) {
        if pk.validate().is_ok() {
            if relay.node_config.public_key() == pk {
                response.peer_node_info = Some(relay.peer_node_info().await?);
                return Ok(response);
            }
            if let Some(pni) = relay.ds.peer_store.query_nodes_peer_node_info(&pk).await? {
                response.peer_node_info = Some(pni);
                return Ok(response);
            }
            let id = PeerId::from_pk(pk);
            if relay.node_config.peer_id() == id {
                response.peer_id_info = Some(relay.peer_id_info().await?);
                return Ok(response);
            }

            if let Some(pid_info) = relay.ds.peer_store
                .query_peer_id_info(&id)
                .await? {
                response.peer_id_info = Some(pid_info);
                return Ok(response);
            }

            // TODO: instead query all address, merge results.
            // if let Ok(addrs) = pk.to_all_addresses_for_network(&relay.node_config.network) {
            //
            // }

        }
    }

    Ok(response)
}

pub async fn get_address_info(relay: &Relay, limit: Option<i64>, offset: Option<i64>, a: &Address) -> Result<AddressInfo, ErrorInfo> {
    let res = relay.ds.transaction_store.query_utxo_address(&a).await?;
    let mut info = AddressInfo::from_utxo_entries(a.clone(), res);
    let limit = limit.unwrap_or(10);
    let offset = offset.unwrap_or(0);
    info.recent_transactions = relay.ds.transaction_store.get_all_tx_for_address(&a, limit, offset).await?;
    Ok(info)
}

// TODO: Generalize this and use new stricter type when utxo entries are not required.
// Also update balance calculation to take into account multiple currencies or products.
pub async fn get_address_info_public_key(relay: &Relay, pk: &PublicKey, limit: Option<i64>, offset: Option<i64>) -> Result<AddressInfo, ErrorInfo> {
    let address = pk.address()?;
    let mut info = get_address_info(relay, limit, offset, &address).await?;
    let address_btc = pk.to_bitcoin_address_typed(&relay.node_config.network)?;
    let eth_info = get_address_info(relay, limit, offset, &address_btc).await?;
    let address_eth = pk.to_ethereum_address_typed()?;
    let btc_info = get_address_info(relay, limit, offset, &address_eth).await?;

    info.balance += eth_info.balance;
    info.balance += btc_info.balance;
    info.recent_transactions.extend(eth_info.recent_transactions);
    info.recent_transactions.extend(btc_info.recent_transactions);
    info.utxo_entries.extend(eth_info.utxo_entries);
    info.utxo_entries.extend(btc_info.utxo_entries);

    Ok(info)
}
