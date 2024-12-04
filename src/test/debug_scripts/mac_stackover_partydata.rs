use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::structs::NetworkEnvironment;
use crate::node_config::{ApiNodeConfig, EnvDefaultNodeConfig};

#[ignore]
#[tokio::test]
async fn debug_query_party_data() {
    let nc = NodeConfig::default_env(NetworkEnvironment::Dev).await;
    let res = nc.api_rg_client().party_data().await.log_error().map(|mut r| {
        r.iter_mut().for_each(|(k, v)| {
            v.party_events.as_mut().map(|pev| {
                pev.portfolio_request_events.enriched_events = Some(pev.portfolio_request_events.calculate_current_fulfillment_by_event());
            });
        });
        r.clone()
    }).unwrap_or_default();
    println!("{:?}", res);

}