use rocket::form::validate::Len;
use redgold_schema::structs::{AboutNodeRequest, AboutNodeResponse, ErrorInfo};
use crate::core::relay::Relay;

pub(crate) async fn handle_about_node(p0: AboutNodeRequest, relay: Relay) -> Result<AboutNodeResponse, ErrorInfo> {
    let num_active_peers = relay.ds.peer_store.active_nodes().await?.len();
    let num_total_peers = relay.ds.peer_store.all_peers().await?.len();
    // relay.ds.transaction_store.query_transaction()
    Ok(AboutNodeResponse{
        latest_metadata: Some(relay.node_config.peer_data_tx()),
        latest_node_metadata: Some(relay.node_config.peer_node_data_tx()),
        num_known_peers: num_total_peers as i64,
        num_active_peers: num_active_peers as i64,
        recent_transactions: vec![],
        pending_transactions: 0,
        total_accepted_transactions: 0,
        observation_height: 0,
    })
}