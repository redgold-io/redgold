use dirs::home_dir;
use redgold_data::data_store::DataStore;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::structs::SupportedCurrency;
use crate::infra::multiparty_backup::parse_mp_csv;
use crate::node_config::{ApiNodeConfig, EnvDefaultNodeConfig};

#[ignore]
#[tokio::test]
async fn debug() {
    let home = home_dir().expect("home");
    let p = home.join(".rg/hetzner/dev/data_store.sqlite");
    let p2 = home.join(".rg/hetzner/dev/multiparty.csv");
    let ds = DataStore::from_config_path(&p).await;
    let info = ds.multiparty_store.all_party_info_with_key().await.unwrap();
    println!("{:?}", info);

    let dat = parse_mp_csv(std::fs::read_to_string(p2).expect("read")).expect("parse");

    // println!("data_parse {:?}", dat);

    for d in dat {
        println!("party key {}", d.party_key.json_or())
    }


}
