use crate::core::relay::Relay;
use redgold_schema::keys::words_pass::WordsPassMetadata;
use redgold_schema::helpers::easy_json::EasyJsonDeser;
#[ignore]
#[tokio::test]
async fn metadata_converter() {
    let r = Relay::dev_default().await;
    let df = r.node_config.secure_data_folder.clone().expect("secure");
    println!("{}", df.path.to_str().expect("path"));
    let all = df.all();
    let all_p = all.path.join("metadata.json");
    println!("{}", all_p.to_str().expect("path"));
    let dat = tokio::fs::read_to_string(all_p).await.expect("metadata");
    let m = dat.json_from::<WordsPassMetadata>().expect("metadata");

    println!("name,derivation_path,xpub");

    for w in m.rdg_btc_message_account_metadata.clone() {
        if w.account >= 50 && w.account < 60 {
            println!("gen_{},{},{}", w.account, w.derivation_path, w.xpub);
        }
    }

    for w in m.rdg_btc_message_account_metadata.iter().rev() {
        if w.account >= 90 && w.account < 100 {
            let inv = 100 - w.account;
            println!("peer_{}_{},{},{}", w.account, inv, w.derivation_path, w.xpub);
        }
    }

}