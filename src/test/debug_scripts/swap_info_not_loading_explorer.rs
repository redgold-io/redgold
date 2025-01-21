use std::collections::HashMap;
use std::fs;
use redgold_data::data_store::DataStore;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::party::party_internal_data::PartyInternalData;
use redgold_schema::structs::{Address, Hash, NetworkEnvironment, Transaction, UtxoId};
use redgold_schema::util::lang_util::AnyPrinter;
use crate::api::explorer::handle_explorer_hash;
use crate::core::relay::Relay;
use crate::node_config::EnvDefaultNodeConfig;
use crate::util;

#[ignore]
#[tokio::test]
async fn dbg_main() {
    let n = NodeConfig::by_env_with_args(NetworkEnvironment::Dev).await;
    let s = n.secure_or().by_env(n.network);
    let r = Relay::new(n).await;
    // r.ds.table_sizes().await.unwrap().json_or().print();
    let h = "0a220a2041e1bc580e996e3fcf95ddb01e7e28f94ba6a61b164d540af6a8642954a460df".to_string();
    let mut hm = HashMap::default();
    for p in r.ds.multiparty_store.all_party_info_with_key().await.unwrap() {
        let pk = p.party_key.unwrap();
        let pd = r.ds.multiparty_store.party_data(&pk).await.unwrap().unwrap();
        let prior_data = r.ds.multiparty_store.party_data(&pk).await.unwrap()
            .and_then(|pd| pd.json_party_internal_data)
            .and_then(|pid| pid.json_from::<PartyInternalData>().ok()).unwrap();
        hm.insert(pk, prior_data);
    }
    r.external_network_shared_data.write(hm).await;
    let res = handle_explorer_hash(h, r, Default::default()).await.unwrap();
    // res.json_or().print();
    let t = res.transaction.unwrap();
    let s = t.swap_info.unwrap();
    s.json_or().print();
}