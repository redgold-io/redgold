use redgold_schema::structs::{AboutNodeRequest, AboutNodeResponse, ErrorInfo};
use crate::core::relay::Relay;

pub async fn handle_about_node(_p0: AboutNodeRequest, relay: Relay) -> Result<AboutNodeResponse, ErrorInfo> {
    let num_active_peers = relay.ds.peer_store.active_nodes(None).await?.len();
    let num_total_peers = relay.ds.peer_store.all_peers().await?.len();
    let recent_transactions = relay.ds.transaction_store.query_recent_transactions(None).await?;
    let total_accepted_transactions =
        relay.ds.transaction_store.count_total_accepted_transactions().await?;
    let pending_transactions = relay.transaction_channels.len() as i64;
    let observation_height = relay.ds.observation.select_latest_observation(
        relay.node_config.public_key()).await?.map(|o| o.height).unwrap_or(0);

    // let mut about = AboutNodeResponse::default();
    // relay.
   let peer_node_info = Some(relay.node_config.self_peer_info());
    Ok(AboutNodeResponse{
        // This should be the peer_id value stored in config data store
        latest_metadata: Some(relay.node_config.peer_data_tx()),
        // This should be the local latest transaction value also stored in config store, but
        // with updates made by the node keypair
        latest_node_metadata: Some(relay.node_config.peer_node_data_tx()),
        num_known_peers: num_total_peers as i64,
        num_active_peers: num_active_peers as i64,
        recent_transactions,
        pending_transactions,
        total_accepted_transactions,
        observation_height,
        peer_node_info,
    })
}