use dirs::home_dir;
use redgold_data::data_store::DataStore;
use redgold_schema::helpers::easy_json::EasyJson;
use crate::api::explorer::handle_explorer_recent;
use crate::api::hash_query::hash_query;
use crate::core::relay::Relay;

#[ignore]
#[tokio::test]
pub async fn debug_endpoints() {
    let home = home_dir().expect("home");
    let ds = home.join("ds.sqlite");
    let ds = DataStore::from_config_path(&ds).await;
    let mut relay = Relay::default().await;
    relay.ds = ds;
    let res = handle_explorer_recent(relay.clone(), None).await.expect("");

    for x in res.recent_observations {
        let res = hash_query(relay.clone(), x.hash, None, None).await.expect("");
        println!("{}", res.json_or())
    }


}