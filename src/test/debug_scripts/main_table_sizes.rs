use crate::node_config::EnvDefaultNodeConfig;
use redgold_data::data_store::DataStore;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::structs::NetworkEnvironment;
use redgold_schema::util::lang_util::AnyPrinter;
use std::fs;

#[ignore]
#[tokio::test]
async fn dbg_main() {
    let n = NodeConfig::by_env_with_args(NetworkEnvironment::Main).await;
    let s = n.secure_or().by_env(n.network);

    // listdir
    let backups = s.backups_ds();
    let files = fs::read_dir(backups).unwrap();
    for f in files {
        let f = f.unwrap();
        let path = f.path();
        path.to_string_lossy().print();
        for i in 0..8 {
            println!("Server num {}", i);
            let this_path = path.clone().join(i.to_string());
            let ds = this_path.join("data_store.sqlite");
            let ds = DataStore::from_config_path(&ds).await;
            let ts = ds.table_sizes().await.unwrap();
            for t in ts {
                println!("Table: {} size MB: {}", t.0, t.1/(1024*1024));
            }
        }
    }
}