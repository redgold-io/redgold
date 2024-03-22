use redgold_keys::transaction_support::TransactionSupport;
use redgold_schema::{EasyJson, ErrorInfoContext, SafeOption};
use redgold_schema::structs::{CurrencyAmount, NetworkEnvironment};
use redgold_schema::util::lang_util::AnyPrinter;
use crate::api::public_api::PublicClient;
use crate::core::transact::tx_builder_supports::{TransactionBuilder, TransactionBuilderSupport};
use crate::node_config::NodeConfig;
use crate::test::external_amm_integration::{amm_public_key, dev_ci_kp};

// Use this for manual testing
#[ignore]
#[tokio::test]
pub async fn double_spend_debug() {

    let network = NetworkEnvironment::Dev;
    let nc = NodeConfig::default_env(network).await;

    let seed = nc.seeds.get(0).cloned().expect("seed address");

    if let Some((_privk, keypair)) = dev_ci_kp() {
        let pk = keypair.public_key();
        let rdg_address = pk.address().expect("");
        println!("pk: {}", rdg_address.render_string().expect(""));
        let api_client = nc.api_client();
        let result = api_client.query_address(
            vec![rdg_address.clone()]).await.expect(""
        ).as_error().expect("");
        let utxos = result.query_addresses_response.safe_get_msg("").expect("")
            .utxo_entries.clone();

        let single_utxo = utxos.get(0).cloned().expect("utxo");
        let single_utxo_amount = single_utxo.amount() as i64;
        let amt = CurrencyAmount::from(single_utxo_amount);
        let mut res = vec![];
        for si in &nc.seeds {
            let sic = si.clone();
            let addr = si.clone().public_key.expect("").address().expect("");
            let s_utxo = single_utxo.clone();

            let tb = TransactionBuilder::new(&nc)
                .with_unsigned_input(s_utxo).expect("utxos")
                .with_output(&addr, &amt)
                .build().expect("build")
                .sign(&keypair).expect("sign");

            let results = tokio::spawn(async move {
                let pc = PublicClient::from(
                    sic.external_address.clone(),
                    sic.port_or(nc.port_offset) + 1,
                    None
                );
                pc.send_transaction(&tb, true).await.json_or()
            });
            res.push(results);
        }
        let results = futures::future::join_all(res).await;

        for x in results {
            x.error_info("Join error").json_or().print();
        }

    }
}
