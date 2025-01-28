use crate::core::relay::Relay;
use crate::infra::deploy::derive_mnemonic_and_peer_id;
use redgold_common_no_wasm::data_folder_read_ext::EnvFolderReadExt;
use redgold_keys::util::mnemonic_support::MnemonicSupport;
use redgold_schema::keys::words_pass::WordsPass;
use redgold_schema::proto_serde::ProtoSerde;
use redgold_schema::structs::NetworkEnvironment;
use std::collections::HashMap;

#[ignore]
#[tokio::test]
async fn dump_seed_info_string() {
    let r = Relay::dev_default().await;
    let df = r.node_config.secure_data_folder.clone().expect("secure");
    let salt_m = df.all().mnemonic().await.expect("mnemonic");
    println!("{:?}", df.path);

    let mut hm = HashMap::new();

    for i in 0..10 {
        let (words, pid) = derive_mnemonic_and_peer_id(&
            r.node_config,
            salt_m.clone(),
            i.clone(),
            false,
            None,
            None,
            i as i64,
            vec![], // only used for PeerData.servers
            vec![],
            &mut hm,
            &NetworkEnvironment::Dev, // not used for pid / mnemonic
            None
        ).await.expect("derive");
        let w = WordsPass::words(words);
        let pk = w.default_kp().expect("kp").public_key();
        let pk_hex = pk.hex();
        println!("simple_seed(\n\"n{i}.redgold.io\",\n\"{pid}\",\n\"{pk_hex}\",\nfalse),");
    }

}