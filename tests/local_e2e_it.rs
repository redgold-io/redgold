
use redgold::api::client::public_client::PublicClient;

use redgold::e2e::tx_submit::TransactionSubmitter;
use redgold::node_config::EnvDefaultNodeConfig;
use redgold::util;
use redgold_schema::conf::node_config::NodeConfig;
use redgold_schema::structs::{ErrorInfo, NetworkEnvironment};
use redgold_schema::SafeOption;


#[tokio::test]
async fn local_e2e_it() -> Result<(), ErrorInfo> {

    redgold_common::log::init_logger_once();

    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    println!("Local E2E IT from inside test");

    let port_offset = NetworkEnvironment::Local.default_port_offset();
    let pc = PublicClient::from("127.0.0.1".to_string(), port_offset + 1 + 1000*10, None);
    let pc2 = PublicClient::from("127.0.0.1".to_string(), port_offset + 1 + 1000*11, None);
    let pc3 = PublicClient::from("127.0.0.1".to_string(), port_offset + 1 + 1000*12,None);

    assert_eq!(pc.client_wrapper().get_peers().await?.get_peers_info_response.safe_get()?.peer_info.len(), 2);
    assert_eq!(pc2.client_wrapper().get_peers().await?.get_peers_info_response.safe_get()?.peer_info.len(), 2);
    assert_eq!(pc3.client_wrapper().get_peers().await?.get_peers_info_response.safe_get()?.peer_info.len(), 2);

    let seeds = pc.client_wrapper().seeds().await.expect("seeds");

    let mut nc = NodeConfig::default_env(NetworkEnvironment::Local).await;
    nc.seeds = seeds.clone();
    let submit = TransactionSubmitter::default(pc.clone(), vec![], &nc);
    submit.with_faucet().await.expect("");

    let res = submit.submit().await.expect("");
    assert!(res.query_transaction_response.is_some());
    Ok(())
    //
    // tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
    //
    //
    // let mut config2 = nc.clone();
    //
    // let mut folders = vec![];
    // let starting_df = config2.data_folder.path.clone();
    // for id in vec![0,1,2] {
    //     let path = starting_df.join("local_test");
    //     let id_path = path.join(format!("id_{}", id));
    //     let mock_resources = id_path.join("mock_resources");
    //     folders.push(mock_resources);
    // }
    //
    // let client =  pc.client_wrapper();
    // let string = pc.client_wrapper().url();
    // info!("setting test harness to {} ", string.clone());
    // info!("active party key {}", client.clone().active_party_key().await.expect("works").json_or());
    // config2.load_balancer_url = string;
    // let vec = vec![];
    // let kp = dev_ci_kp().expect("kp").1;
    //
    // let mut amm_test_harness = PartyTestHarness::from(
    //     &config2, kp, vec, Some(client.clone()), folders).await;
    //
    // let address = amm_test_harness.self_rdg_address();
    // submit.send_to(&address).await.expect("works");
    // // submit.send_to(&address).await.expect("works");
    // // submit.send_to(&address).await.expect("works");
    //
    // let b = client.clone().balance(&address).await.expect("works");
    // info!("Balance: {}", b.json_or());
    // amm_test_harness.run_test().await.expect("works");
    //
    // Ok(())
    //

    //
    // // TODO: Change the runtime structure to implement the shutdowns directly inside, then pass
    // // around the Arc reference externally for spawning.
    // let rt = build_simple_runtime(1, "test_run_main");
    // let args = RgArgs::default();
    // rt.spawn_blocking(move || main_from_args(args));
    // sleep(Duration::from_secs(10));
    // rt.shutdown_timeout(Duration::from_secs(10));
}