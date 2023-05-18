use redgold_schema::structs::{AboutNodeRequest, AboutNodeResponse, ErrorInfo};
use crate::core::relay::Relay;

pub async fn handle_about_node(p0: AboutNodeRequest, relay: Relay) -> Result<AboutNodeResponse, ErrorInfo> {
    let num_active_peers = relay.ds.peer_store.active_nodes(None).await?.len();
    let num_total_peers = relay.ds.peer_store.all_peers().await?.len();
    let peers_info = relay.ds.peer_store.peer_node_info().await?;
    let recent_transactions = relay.ds.transaction_store.query_recent_transactions(None).await?;
    let total_accepted_transactions =
        relay.ds.transaction_store.count_total_accepted_transactions().await?;
    let pending_transactions = relay.transaction_channels.len() as i64;
    let observation_height = relay.ds.observation.select_latest_observation(
        relay.node_config.public_key()).await?.map(|o| o.height).unwrap_or(0);
    
    // let mut about = AboutNodeResponse::default();
    // relay.
    Ok(AboutNodeResponse{
        self_peer_info: Some(relay.node_config.self_peer_info()),
        num_known_peers: num_total_peers as i64,
        num_active_peers: num_active_peers as i64,
        recent_transactions,
        pending_transactions,
        total_accepted_transactions,
        observation_height,
        peers_info: vec![],
    })
}