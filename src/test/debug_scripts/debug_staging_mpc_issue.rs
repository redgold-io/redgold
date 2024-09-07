use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::structs::{Address, Hash, NetworkEnvironment, UtxoId};
use redgold_schema::util::lang_util::AnyPrinter;
use crate::core::relay::Relay;
use crate::util;

#[ignore]
#[tokio::test]
async fn debug_staging_issue() {
    let r = Relay::env_default(NetworkEnvironment::Staging).await;
    let parties = r.ds.multiparty_store.all_party_info_with_key().await;
    parties.unwrap().json_pretty_or().print();

}