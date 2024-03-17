use std::collections::{HashMap, HashSet};
use bdk::Utxo;
use bytes::Buf;
use itertools::Itertools;
use redgold_schema::{EasyJson, WithMetadataHashable};
use redgold_schema::structs::{TransactionEntry, UtxoEntry, UtxoId};
use crate::core::relay::Relay;
use crate::util;

#[ignore]
#[tokio::test]
async fn historical_parity_debug() {
    let r = Relay::dev_default().await;
    let start = util::current_time_millis_i64();
    let mut all_txs = r.ds.transaction_store.query_time_transaction(0, start).await.unwrap();
    all_txs.sort_by(|a, b| a.time.cmp(&b.time));
    let end = util::current_time_millis_i64();
    let delta = (end - start) as f64;
    println!("delta: {}", delta/1000f64);
    println!("res: {:?}", all_txs.len());
    println!("{}", all_txs.get(0).unwrap().json_or());
    let mut valid_utxos: HashMap<UtxoId, UtxoEntry> = Default::default();

    validate_utxos(&mut all_txs, valid_utxos);

}

fn validate_utxos(all_txs: &mut Vec<TransactionEntry>, mut valid_utxos: HashMap<UtxoId, UtxoEntry>) {
    let gen = all_txs.get(0).unwrap().clone();
    let gen_tx = gen.transaction.expect("tx");
    let vec = gen_tx.utxo_outputs().expect("works");
    for utxo_entry in vec {
        let id = utxo_entry.utxo_id.clone().unwrap();
        valid_utxos.insert(id.clone(), utxo_entry.clone());
    };

    println!("gen_tx: {:?}", gen_tx.json_or());
    // Verify time greater for all children too?
    let mut validated_count = 0;
    let total = all_txs.len();

    for (idx, t) in all_txs.iter().dropping(1).enumerate() {
        if let Some(t) = t.transaction.as_ref() {
            let has_amount = t.output_amounts_opt().filter(|&a| a.amount > 0).next().is_some();
            if !has_amount {
                println!("no amount: skipping tx {}", t.json_or());
                continue;
            }
            validated_count += 1;
            let fraction = (idx as f64 / total as f64)*100f64;
            if idx % 1000 == 0 {
                println!("idx {}, fraction: {} validated_count: {}", idx, fraction, validated_count);
            }
            let time = t.time().expect("time");
            t.input_utxo_ids().for_each(|utxo_id| {
                assert!(valid_utxos.contains_key(utxo_id));
                let prev = valid_utxos.get(utxo_id).expect("prev");
                let prev_time = prev.time;
                assert!(time.clone() > prev_time);
                valid_utxos.remove(utxo_id);
            });
            let outputs = t.utxo_outputs().expect("outputs");
            for utxo_entry in outputs {
                let utxo_id = utxo_entry.utxo_id.clone().expect("utxo_id");
                assert!(!valid_utxos.contains_key(&utxo_id));
                valid_utxos.insert(utxo_id.clone(), utxo_entry.clone());
            };
        }
    }
}