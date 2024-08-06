use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::structs::{Address, Hash, UtxoId};
use redgold_schema::util::lang_util::AnyPrinter;
use crate::core::relay::Relay;
use crate::util;

#[ignore]
#[tokio::test]
async fn debug_balance_issue() {
    let r = Relay::dev_default().await;

    let fulfillment_hash = Hash::from_hex("0a220a20dbafd3e7aa9432909a86a5a6bf6094db45fc52e9e1b13132efee02b7dd6bb1b3")
        .unwrap();

    let addr_str = "tb1q60689u58clr3guk964nfutnuznktp3d4ld6khg".to_string();
    let addr = Address::from_bitcoin(&addr_str);
    // let res = r.ds.utxo.utxo_for_address(&addr).await.unwrap();
    // res.json_pretty_or().print();
    // let child_idx = r.ds.utxo.query_utxo_output_index(&fulfillment_hash).await.unwrap();
    // child_idx.json_pretty_or().print();
    //
    // r.ds.transaction_store.query_maybe_transaction(&fulfillment_hash).await.unwrap().json_pretty_or().print();
    // r.ds.utxo.utxo_for_id(&UtxoId::new(&fulfillment_hash, 0)).await.unwrap().json_pretty_or().print();
    // addr.json_pretty_or().print();

    for u in r.ds.utxo.utxo_all_debug().await.unwrap() {
        if let Ok(a) = u.address() {
            if a.currency != 0 {
                println!("problem utxo");
                a.json_pretty_or().print();
                a.render_string().unwrap().print();
                u.opt_amount().json_pretty_or().print();
            }
        }
    }

    // r.ds.config_store.get_genesis().await.unwrap().unwrap().json_pretty_or().print()

    let all_tx = r.ds.transaction_store.query_time_transaction_accepted_ordered(0, util::current_time_millis_i64())
        .await
        .unwrap();
    for t in all_tx {
        for x in t.outputs {
            if let Some(a) = x.address {
                if a.currency != 0 {
                    println!("problem txo");
                    a.json_pretty_or().print();
                    a.render_string().unwrap().print();
                }
            }
        }
    }


}