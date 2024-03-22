

// scp root@n1.redgold.io:~/.rg/dev/data_store.sqlite ~/.rg/dev/

use std::collections::HashMap;
use bdk::bitcoin::hashes::hex::ToHex;
use itertools::Itertools;
use log::error;
use redgold_data::data_store::DataStore;
use redgold_keys::KeyPair;
use redgold_keys::util::mnemonic_support::WordsPass;
use redgold_schema::{EasyJson, RgResult};
use redgold_schema::structs::{Address, ErrorInfo, UtxoId};
use redgold_schema::transaction::amount_to_raw_amount;
use crate::core::relay::Relay;
use crate::e2e::LiveE2E;
use crate::e2e::tx_gen::SpendableUTXO;
use crate::node_config::NodeConfig;
use crate::util::cli::arg_parse_config::ArgTranslate;


pub async fn get_error_utxo_ids(ds: &DataStore, map: HashMap<Address, KeyPair>) -> RgResult<Vec<UtxoId>> {
    let mut err_utxo_ids = vec![];
    for (a, k) in map.iter() {
        let result = ds.transaction_store.query_utxo_address(a).await?;
        let vec = result.iter().filter(|r| r.amount() > amount_to_raw_amount(1)).collect_vec();
        let mut err_str = format!("Address {}", a.render_string().expect(""));
        for u in vec {
            if let Ok(id) = u.utxo_id() {
                err_str.push_str(&format!(" UTXO ID: {}", id.json_or()));
                if ds.utxo.utxo_id_valid(id).await? {
                    let childs = ds.utxo.utxo_children(id).await?;
                    if childs.len() == 0 {

                    } else {
                        error!("UTXO has children not valid! {} children: {}", err_str, childs.json_or());
                        err_utxo_ids.push(id.clone());
                    }
                } else {
                    error!("UTXO ID not valid! {}", err_str);
                }
            }
        }
    }
    Ok(err_utxo_ids)
}


#[ignore]
#[tokio::test]
async fn debug_live_e2e_utxos() {
    if let Some(sec) = ArgTranslate::secure_data_path_buf() {
        let r = Relay::dev_default().await;

        let pb = sec.join("hn_all_words");
        let words = tokio::fs::read_to_string(pb).await.unwrap();
        let wp = WordsPass::words(words);
        wp.validate().expect("valid");

        println!("hex pk {}", wp.default_kp().expect("").public_key.to_hex());

        let map = LiveE2E::live_e2e_address_kps(
            &wp, &r.node_config.network
        ).expect("");
        // let spendable_utxos = LiveE2E::get_spendable_utxos(&r.ds, map).await.expect("");

        let errs = get_error_utxo_ids(&r.ds, map).await.expect("");

        for e in errs {
            println!("Error UTXO: {}", e.json_or());
        }


        // let destination_choice = r.node_config.seed_addresses().get(0).cloned().expect("seed address");
        // let tx = LiveE2E::build_live_tx(&r.node_config, destination_choice, spendable_utxos).expect("");


    }
}