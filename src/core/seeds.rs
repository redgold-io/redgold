use bdk::bitcoin::secp256k1::PublicKey;
use redgold_schema::seeds::get_seeds;
use redgold_schema::structs::{NetworkEnvironment, TrustData};
use crate::api::public_api::PublicClient;
use crate::api::RgHttpClient;
use crate::node_config::NodeConfig;

#[ignore]
#[tokio::test]
pub async fn debug_get_seeds_info() {
    let s = get_seeds();
    let mut nc = NodeConfig::default();
    nc.network = NetworkEnvironment::Dev;
    for si in s {
        let pc = PublicClient::from(si.external_address.clone(), nc.public_port(), None);
        let a = pc.about().await.expect("about");
        let pni = a.peer_node_info.expect("pni");
        let nmd = pni.latest_node_transaction.expect("node").node_metadata().expect("");
        let res = nmd.public_key.expect("").hex_or();
        let pidhex = nmd.peer_id.expect("").peer_id.expect("").hex_or();
        println!("s({}, {}, {});", si.external_address, pidhex, res);
    }

}