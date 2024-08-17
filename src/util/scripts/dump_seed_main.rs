use std::fs;
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::{Address, Hash, NetworkEnvironment, Transaction, UtxoId};
use redgold_schema::util::lang_util::AnyPrinter;
use crate::core::relay::Relay;
use crate::util;

#[ignore]
#[tokio::test]
async fn dbg_main() {
    let r = Relay::env_default(NetworkEnvironment::Dev).await;
    let p = r.node_config.secure_path().unwrap();
    for i in 0..8 {
        let path = format!("{}/.rg/all/offline_servers_info/{}/peer_tx", p.clone(), i);
        // path.print();
        // read the file
        let file = fs::read_to_string(path).unwrap();
        let t = file.json_from::<Transaction>().expect("Failed to parse json");
            // .json_pretty_or().print();
        let metadata = t.peer_data().expect("p");
        let pid = metadata.peer_id.clone();
        let pid = pid.unwrap().hex();
        let pk_hex = metadata.node_metadata.get(0).unwrap().clone().public_key.unwrap().hex();
        println!("simple_seed(\n\"n{i}.redgold.io\",\n\"{pid}\",\n\"{pk_hex}\",\ntrue),");
    }

}