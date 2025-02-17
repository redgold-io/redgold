use crate::core::relay::Relay;
use crate::util;
use itertools::Itertools;
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::helpers::with_metadata_hashable::WithMetadataHashable;
use redgold_schema::structs::{Hash, Transaction, UtxoEntry, UtxoId};
use redgold_schema::util::times::ToTimeString;
use std::collections::HashMap;


#[ignore]
#[tokio::test]
async fn historical_parity_debug() {
    let r = Relay::dev_default().await;
    let start = util::current_time_millis_i64();
    let mut all_txs = r.ds.transaction_store.query_time_transaction_accepted_ordered(0, start).await.unwrap();
    all_txs.sort_by(|a, b| a.time().expect("t").cmp(&b.time().expect("")));
    let end = util::current_time_millis_i64();
    let delta = (end - start) as f64;
    println!("delta: {}", delta/1000f64);
    println!("res: {:?}", all_txs.len());
    println!("{}", all_txs.get(0).unwrap().json_or());
    let valid_utxos: HashMap<UtxoId, UtxoEntry> = Default::default();

    validate_utxos(&mut all_txs, valid_utxos);

}

#[ignore]
#[tokio::test]
async fn historical_parity_debug2() {
    let r = Relay::dev_default().await;
    let tx_hash = Hash::from_hex("3780424d7c3f351706529f999c923189426d9ce65aea00af34270c663b4baf12").unwrap();
    let res = r.ds.transaction_store.query_maybe_transaction(&tx_hash).await.unwrap().unwrap();
    let tx = res.0;
    let utxo_id = tx.inputs.get(0).unwrap().utxo_id.clone().unwrap();
    let valid_utxo = r.ds.utxo.utxo_id_valid(&utxo_id).await.unwrap();
    println!("tx: {}", tx.json_or());
    println!("utxo_id: {}", utxo_id.json_or());
    println!("valid_utxo: {}", valid_utxo.json_or());
    let children = r.ds.utxo.utxo_children(&utxo_id).await.unwrap();
    println!("children: {}", children.json_or());
    let child = children.get(0).unwrap();
    let child_tx = r.ds.transaction_store.query_maybe_transaction(&child.0).await.unwrap().unwrap().0;
    println!("child_tx: {}", child_tx.json_or());

    for o in tx.utxo_outputs().unwrap().iter() {
        let u = o.utxo_id.clone().expect("utxo_id");
        let valid_utxo = r.ds.utxo.utxo_id_valid(&u).await.unwrap();
        println!("utxo_id: {}", u.json_or());
    }
}


#[ignore]
#[tokio::test]
async fn historical_parity_debug_cached() {
    let r = Relay::dev_default().await;
    let bad_txs = tokio::fs::read_to_string("bad_txs.json").await.unwrap().json_from::<Vec<Transaction>>()
        .unwrap();

    let bad_txs_unique = bad_txs.iter().unique_by(|tx| tx.hash_or()).collect_vec();

    let mut children_to_explore = vec![];

    for tx in bad_txs_unique.iter() {
        let h = tx.hash_or().hex();
        println!("tx hash {}", h);
        println!("tx time {}", tx.time().expect("").to_time_string());
        for i in tx.inputs.iter() {
            let s = i.address().expect("").render_string().expect("");
            println!("input address: {s}");
            let input_utxoid = i.utxo_id.clone().expect("utxo_id");
            let db_child = r.ds.utxo.utxo_children(&input_utxoid).await.unwrap();
            let u = input_utxoid.json_or();
            println!("input utxo_id: {}", u);
            println!("input actual_child: {}", db_child.json_or());
        }
        for (output_index, o) in tx.outputs.iter().enumerate() {
            let utxo_id = UtxoId::new(&tx.hash_or(), output_index as i64);
            let valid_utxo = r.ds.utxo.utxo_id_valid(&utxo_id).await.unwrap();
            let option = r.ds.utxo.utxo_child(&utxo_id).await.unwrap();
            if let Some(child) = &option {
                children_to_explore.push(child.0.clone());
            }
            let child = option.json_or();
            let output_addr = o.address.as_ref().expect("").render_string().unwrap_or("MISSING ADDRESS OUTPUT".to_string());
            let amt = o.opt_amount();
            let is_swap = o.is_swap();
            let is_liquidity = o.is_stake();
            let data = o.data.clone().expect("");
            let ext = o.response().and_then(|r| r.swap_fulfillment.as_ref())
                .and_then(|r| r.external_transaction_id.as_ref()).json_or();

            println!("{valid_utxo} valid {output_addr} amount {:?} is_swap {is_swap} is_liquidity {is_liquidity} external_id {ext} child {child}", amt);
        }


        println!("--------------------")
    }

    println!("bad_txs: {:?}", bad_txs.len());
    println!("bad_txs_unique: {:?}", bad_txs_unique.len());

    println!("children_to_explore_len: {:?}", children_to_explore.len());
    let children_to_explore_unique = children_to_explore.iter().unique().cloned().collect_vec();

    let mut remaining = vec![];
    children_to_explore_unique.iter().for_each(|c| {
        remaining.push(c.clone());
    });

    let mut all = vec![];
    all.extend(children_to_explore_unique);
    all.extend(bad_txs_unique.iter().map(|tx| tx.hash_or().clone()).collect_vec());

    while remaining.len() > 0 {
        let entry = remaining.pop();
        if let Some(c) = entry {
            let tx = r.ds.transaction_store.query_maybe_transaction(&c).await.unwrap().unwrap().0;
            for u in tx.output_utxo_ids() {
                let option = r.ds.utxo.utxo_child(&u).await.unwrap();
                if let Some(child) = &option {
                    if !remaining.contains(&child.0) {
                        remaining.push(child.0.clone());
                        all.push(child.0.clone())
                    }
                }
            }
        }
    }

    let all_unique = all.iter().unique().cloned().collect_vec();

    println!("all tx: {:?}", all_unique.len());

    all_unique.write_json("all_unique.json").expect("write_json");

    // let tx_hash = Hash::from_hex("3780424d7c3f351706529f999c923189426d9ce65aea00af34270c663b4baf12").unwrap();
    // let res = r.ds.transaction_store.query_maybe_transaction(&tx_hash).await.unwrap().unwrap();
    // let tx = res.0;
    // let utxo_id = tx.inputs.get(0).unwrap().utxo_id.clone().unwrap();
    // let valid_utxo = r.ds.utxo.utxo_id_valid(&utxo_id).await.unwrap();
    // println!("tx: {}", tx.json_or());
    // println!("utxo_id: {}", utxo_id.json_or());
    // println!("valid_utxo: {}", valid_utxo.json_or());
    // let children = r.ds.utxo.utxo_children(&utxo_id).await.unwrap();
    // println!("children: {}", children.json_or());
    // let child = children.get(0).unwrap();
    // let child_tx = r.ds.transaction_store.query_maybe_transaction(&child.0).await.unwrap().unwrap().0;
    // println!("child_tx: {}", child_tx.json_or());
    //

}


fn validate_utxos(all_txs: &mut Vec<Transaction>, mut valid_utxos: HashMap<UtxoId, UtxoEntry>) {
    let gen = all_txs.get(0).unwrap().clone();
    let gen_tx = gen;  // gen.transaction.expect("tx");
    let vec = gen_tx.utxo_outputs().expect("works");
    for utxo_entry in vec {
        let id = utxo_entry.utxo_id.clone().unwrap();
        valid_utxos.insert(id.clone(), utxo_entry.clone());
    };

    println!("gen_tx: {:?}", gen_tx.json_or());
    // Verify time greater for all children too?
    let mut validated_count = 0;
    let total = all_txs.len();

    let mut bad_txs = vec![];

    for (idx, t) in all_txs.iter_mut().dropping(1).enumerate() {
            t.with_hash();

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
            for utxo_id in t.input_utxo_ids() {
                if !valid_utxos.contains_key(utxo_id) {
                    println!("failure on transaction input utxo_id: {} for tx {}", utxo_id.json_or(), t.json_or());
                    bad_txs.push(t.clone());
                } else {
                    let prev = valid_utxos.get(utxo_id).expect("prev");
                    let prev_time = prev.time;
                    let correct_time = time.clone() > prev_time;
                    if !correct_time {
                        println!("failure of time on transaction input utxo_id: {} for tx {}", utxo_id.json_or(), t.json_or());
                        bad_txs.push(t.clone());
                    }
                }
                valid_utxos.remove(utxo_id);
            };
            let outputs = t.utxo_outputs().expect("outputs");
            for utxo_entry in outputs {
                let utxo_id = utxo_entry.utxo_id.clone().expect("utxo_id");
                let in_valid_set_already = valid_utxos.contains_key(&utxo_id);
                if in_valid_set_already {
                    println!("failure on transaction output utxo_id: {} for tx {}", utxo_id.json_or(), t.json_or());
                    bad_txs.push(t.clone());
                }
                valid_utxos.insert(utxo_id.clone(), utxo_entry.clone());
            };
    }

    bad_txs.write_json("bad_txs.json").expect("write_json");

}


#[ignore]
#[tokio::test]
async fn historical_parity_detect_duplicate_hashes() {
    let r = Relay::dev_default().await;
    let start = util::current_time_millis_i64();
    let mut all_txs = r.ds.transaction_store.query_time_transaction_accepted_ordered(0, start).await.unwrap();
    all_txs.sort_by(|a, b| a.time().expect("").cmp(&b.time().expect("")));
    let end = util::current_time_millis_i64();
    let delta = (end - start) as f64;
    println!("delta: {}", delta/1000f64);
    println!("res: {:?}", all_txs.len());

    let dedupe = all_txs.iter().unique_by(|tx|
        tx.hash_or()
    ).count();
    println!("res dedupe: {:?}", dedupe);
    // duplicate_hash_check(&all_txs);

}



#[ignore]
#[tokio::test]
async fn historical_parity_utxo() {
    let r = Relay::dev_default().await;
    let start = util::current_time_millis_i64();
    let mut all_utxo = r.ds.utxo.utxo_all_debug().await.expect("utxo_all_debug");
    all_utxo.sort_by(|a, b| a.time.cmp(&b.time));
    let end = util::current_time_millis_i64();
    let delta = (end - start) as f64;
    println!("delta: {}", delta/1000f64);
    println!("res: {:?}", all_utxo.len());

    let mut is_valid_but_has_kids = vec![];

    for utxo in all_utxo {
        let id = utxo.utxo_id.as_ref().expect("utxo_id");
        let valid = r.ds.utxo.utxo_id_valid(id).await.expect("utxo_id_valid");
        let children = r.ds.utxo.utxo_children(id).await.expect("utxo_children");
        if children.len() > 0 {
            println!("utxo_id: {} valid: {} children: {}", id.json_or(), valid, children.json_or());
            is_valid_but_has_kids.push(id.clone());
        }

    }

    is_valid_but_has_kids.write_json("is_valid_but_has_kids.json").expect("write_json");

    println!("is_valid_but_has_kids: {:?}", is_valid_but_has_kids.len());
}
