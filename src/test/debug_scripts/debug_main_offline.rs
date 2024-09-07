use std::fs;
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::structs::{Address, Hash, NetworkEnvironment, Transaction, UtxoId};
use redgold_schema::util::lang_util::AnyPrinter;
use crate::core::relay::Relay;
use crate::util;

#[ignore]
#[tokio::test]
async fn dbg_main() {
    let r = Relay::env_default(NetworkEnvironment::Dev).await;
    let p = r.node_config.secure_path().unwrap();
    let path = p + "/.rg/all/offline_servers_info/0/peer_tx";
    path.print();
    // read the file
    let file = fs::read_to_string(path).unwrap();
    file.json_from::<Transaction>().expect("Failed to parse json").json_pretty_or().print();

}