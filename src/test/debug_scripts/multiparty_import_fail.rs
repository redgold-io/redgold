use dirs::home_dir;
use redgold_data::data_store::DataStore;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::structs::SupportedCurrency;
use crate::node_config::{ApiNodeConfig, EnvDefaultNodeConfig};

#[ignore]
#[tokio::test]
async fn debug() {
    let home = home_dir().expect("home");
    let p = home.join(".rg/hetzner/dev/data_store.sqlite");
    let ds = DataStore::from_config_path(&p).await;
    let info = ds.multiparty_store.all_party_info_with_key().await.unwrap();
    println!("{:?}", info);


}
