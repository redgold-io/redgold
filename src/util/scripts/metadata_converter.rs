use std::collections::HashMap;
use redgold_keys::util::mnemonic_support::{WordsPass, WordsPassMetadata};
use redgold_schema::helpers::easy_json::EasyJsonDeser;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::NetworkEnvironment;
use crate::core::relay::Relay;
use crate::infra::deploy::derive_mnemonic_and_peer_id;


#[ignore]
#[tokio::test]
async fn metadata_converter() {
    let r = Relay::dev_default().await;
    let df = r.node_config.secure_data_folder.clone().expect("secure");
    let all = df.all();
    let dat = tokio::fs::read_to_string(all.path.join("metadata.json")).await.expect("metadata");
    let m = dat.json_from::<WordsPassMetadata>().expect("metadata");

    for w in m.rdg_btc_message_account_metadata {
        println!("{}", w.account);
        println!("{}", w.rdg_address);
    }
}