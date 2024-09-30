use redgold_schema::util::lang_util::AnyPrinter;
use crate::core::relay::Relay;

#[ignore]
#[tokio::test]
async fn db_table_size() {
    let r = Relay::dev_default().await;
    for a in r.ds.table_size("utxo").await.expect("table sizes") {
        println!("Size: {}", a);
    };
    for (t, s) in r.ds.table_sizes().await.expect("") {
        println!("Table: {}, Size: {}", t, s);
    }
}