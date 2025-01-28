use crate::infra::multiparty_backup::parse_mp_csv;
use crate::integrations::external_network_resources::ExternalNetworkResourcesImpl;
use crate::multiparty_gg20::initiate_mp::default_room_id_signing;
use crate::node_config::EnvDefaultNodeConfig;
use crate::test::external_amm_integration::dev_ci_kp;
use crate::test::local_test_context::{LocalNodes, LocalTestNodeContext};
use crate::util;
use crate::util::cli::arg_parse_config::ArgTranslate;
use crate::util::cli::load_config::load_full_config;
use crate::util::runtimes::{big_thread, build_simple_runtime};
use itertools::Itertools;
use log::info;
use redgold_common::external_resources::{EncodedTransactionPayload, ExternalNetworkResources};
use redgold_keys::address_external::ToEthereumAddress;
use redgold_keys::proof_support::ProofSupport;
use redgold_rpc_integ::eth::eth_wallet::EthWalletWrapper;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::Loggable;
use redgold_schema::structs::StandardContractType::Currency;
use redgold_schema::structs::{Address, ControlMultipartySigningRequest, CurrencyAmount, Hash, InitiateMultipartySigningRequest, NetworkEnvironment, PartyInfo, PartyPurpose, Proof, SupportedCurrency};
use redgold_schema::{bytes_data, RgResult, SafeOption};
use std::sync::Arc;


pub async fn mp_util_send_eth(destination: &Address, amount: &CurrencyAmount, net: NetworkEnvironment) -> RgResult<()> {

    let nc = NodeConfig::by_env_with_args(net.clone()).await;

    let mut ext = ExternalNetworkResourcesImpl::new(&nc, None).unwrap();

    let rows = get_mp_rows();

    let party_public_key = rows.get(0)
        .unwrap().party_key.as_ref().unwrap();
    let mp_eth_addr = party_public_key.to_ethereum_address_typed()
        .unwrap();

    let gas_override = CurrencyAmount::from_eth_bigint_string("111151412793670539"); // 412793670539 default
    let eth = ext.eth_dummy_wallet().await.unwrap();
    let gas = eth.get_gas_price().await.unwrap();
    // eth.create_transaction_typed_inner()
    // eth.get_gas_cost_estimate()
    println!("Gas price: {}", gas.to_fractional());
    // println!("Default Gas price: {}", CurrencyAmount::gas_price_fixed_normal_by_env(&net).to_fractional());

    // let gas = Some(gas_override);
    let gas = None;
    let (data, valid, tx_ser) = ext.eth_tx_payload(
        &mp_eth_addr, &destination, &amount, gas).await
        .unwrap();

    let returned_proof = multiparty_manual_rebuilt_proof(rows.clone(), data);

    let sig = returned_proof.signature.ok_msg("Missing keysign result signature")?;
    let raw = EthWalletWrapper::process_signature_ser(sig, tx_ser, eth.chain_id, nc.network.is_main_stage_network()).unwrap();
    let txid = ext.broadcast(&party_public_key, SupportedCurrency::Ethereum, EncodedTransactionPayload::BytesPayload(raw.to_vec())).await.unwrap();
    info!("Broadcast txid: {}", txid);
    Ok(())
}

#[ignore]
#[tokio::test]
pub async fn utility_keysign_test() {
    util::init_logger_once();
    //
    // let returned_proof = test_manual_sign();
    // info!("Proof: {:?}", returned_proof);
    //
    // let addr = "0xF041ef158993755495fD1c2C1B76C401FAf8718B".to_string();
    // let addr = Address::from_eth_external(&addr);
    // let amount = CurrencyAmount::from_eth_fractional(0.01);
    // mp_util_send_eth(&addr, &amount, NetworkEnvironment::Dev).await.expect("sent");
    let (a, kp) = dev_ci_kp().unwrap();
    let eth = kp.public_key().to_ethereum_address_typed().unwrap();
    println!("Eth: {}", eth.render_string().unwrap());

}

fn test_manual_sign() {
    let rows = get_mp_rows();
    let signing_data = Hash::digest("test".as_bytes().to_vec());
    let signing_data_vec = signing_data.raw_bytes().expect("works");
    let returned_proof = multiparty_manual_rebuilt_proof(rows, signing_data_vec);
    returned_proof.verify(&signing_data).expect("verified");
    info!("Proof: {:?}", returned_proof);
}

fn multiparty_manual_rebuilt_proof(rows: Vec<PartyInfo>, signing_data_vec: Vec<u8>) -> Proof {

    let returned_proof = big_thread().spawn(move || {
        let runtime = build_simple_runtime(num_cpus::get(), "config");
        let ret = runtime.block_on(async {
            let mut local_nodes = LocalNodes::new(None).await;
            for i in 0..7 {
                local_nodes.add_node().await;
            }

            let pks = local_nodes.nodes.iter()
                .map(|n| n.node.relay.node_config.public_key.clone()).collect_vec();

            let mut row = rows.get(0).unwrap().clone();
            let init = row.initiate.as_mut().unwrap();
            init.set_purpose(PartyPurpose::DebugPurpose);
            init.identifier.as_mut().unwrap().party_keys = pks.clone();

            let identifier = init.identifier.clone().expect("identifier");

            let vec = bytes_data(signing_data_vec.clone()).expect("");
            let mut signing_request = ControlMultipartySigningRequest::default();
            let mut init_signing = InitiateMultipartySigningRequest::default();
            let signing_id = default_room_id_signing(&identifier.room_id.clone().expect("rid")).ok();
            init_signing.signing_room_id = signing_id;
            init_signing.data_to_sign = Some(vec);
            init_signing.identifier = Some(identifier.clone());
            signing_request.signing_request = Some(init_signing.clone());
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            for (ni, n) in local_nodes.nodes.iter().enumerate() {
                let mut row = rows.get(ni).expect("row").clone();
                let init = row.initiate.as_mut().unwrap();
                init.set_purpose(PartyPurpose::DebugPurpose);
                init.identifier.as_mut().unwrap().party_keys = pks.clone();
                let init_keygen = init.clone();
                let row2 = row.clone();
                n.node.relay.ds.multiparty_store.add_keygen(&row2).await.expect("add keygen");
                info!("Authorizing room keygen id {}", init_keygen.identifier.as_ref().unwrap().room_id.as_ref().unwrap().clone().json_or());
                n.node.relay.authorize_keygen(init_keygen).expect("authorized");
                info!("Authorizing room signing id {}", init_signing.signing_room_id.json_or());
                n.node.relay.authorize_signing(init_signing.clone()).expect("authorized");
                let pi = init_signing.identifier.as_ref().unwrap().party_index(&n.node.relay.node_config.public_key).unwrap();
                info!("Party index: {}", pi);
            }

            let ccl = local_nodes.start().clone().control_client;

            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            let res = ccl.multiparty_signing(signing_request).await.log_error().expect("signing");
            // println!("{:?}", res);
            let proof = res.proof.expect("prof");
            // proof.verify(&signing_data).expect("verified");
            proof
        });
        ret
    }).unwrap().join().unwrap();
    returned_proof
}

fn get_mp_rows() -> Vec<PartyInfo> {
    let rows = big_thread().spawn(|| {
        let runtime = build_simple_runtime(num_cpus::get(), "config");
        let ret = runtime.block_on(get_shares());
        runtime.shutdown_background();
        ret
    }).unwrap().join().unwrap();
    rows
}

async fn get_shares() -> Vec<PartyInfo> {
    let nc = NodeConfig::by_env_with_args(NetworkEnvironment::Dev).await;

    let latest = crate::infra::multiparty_backup::get_backup_latest_path(nc.clone()).await.expect("latest").expect("latest");
    println!("Latest backup: {:?}", latest.clone());

    let mut rows = vec![];

    for i in 0..8 {
        let mp_csv = latest.join(i.to_string());
        let mp_csv = mp_csv.join("multiparty.csv");
        println!("Reading multiparty csv: {:?}", mp_csv);

        let raw = tokio::fs::read_to_string(mp_csv).await.expect("read mp csv");

        let result = parse_mp_csv(raw);
        for row in result.expect("parsed") {
            rows.push(row);
            // let h = Hash::digest(row.clone().proto_serialize()).checksum_hex();
            // let local_keyhash = row.local_key_share.unwrap().proto_serialize_hex();
            // let pk = row.party_key.unwrap().proto_serialize_hex().last_n(10);
            // println!("pk {} local {}", pk, h);
        }
    }
    rows
}
