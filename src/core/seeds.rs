use crate::api::client::public_client::PublicClient;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::seeds::get_all_hardcoded_seeds;
use redgold_schema::structs::NetworkEnvironment;

#[ignore]
#[tokio::test]
pub async fn debug_get_seeds_info() {
    let s = get_all_hardcoded_seeds();
    let mut nc = NodeConfig::default();
    nc.network = NetworkEnvironment::Dev;
    for si in s {
        let pc = PublicClient::from(si.external_address.clone(), nc.public_port(), None);
        let a = pc.about().await.expect("about");
        let pni = a.peer_node_info.expect("pni");
        let nmd = pni.latest_node_transaction.expect("node").node_metadata().expect("");
        let res = nmd.public_key.expect("").hex();
        let pidhex = nmd.peer_id.expect("").peer_id.expect("").hex();
        println!("s({}, {}, {});", si.external_address, pidhex, res);
    }

}