



use rocket::form::validate::Len;

use redgold::api::public_api::PublicClient;

use redgold::e2e::tx_submit::TransactionSubmitter;




use redgold_schema::{SafeOption};
use redgold_schema::structs::{ErrorInfo, NetworkEnvironment};


#[tokio::test]
async fn local_e2e_it() -> Result<(), ErrorInfo> {

    // util::init_logger();;
    println!("Local E2E IT from inside test");

    let port_offset = NetworkEnvironment::Local.default_port_offset();
    let pc = PublicClient::from("127.0.0.1".to_string(), port_offset + 1, None);
    let pc2 = PublicClient::from("127.0.0.1".to_string(), port_offset + 1 + 1000, None);
    let pc3 = PublicClient::from("127.0.0.1".to_string(), port_offset + 1 + 2000,None);

    assert_eq!(pc.client_wrapper().get_peers().await?.get_peers_info_response.safe_get()?.peer_info.len(), 2);
    assert_eq!(pc2.client_wrapper().get_peers().await?.get_peers_info_response.safe_get()?.peer_info.len(), 2);
    assert_eq!(pc3.client_wrapper().get_peers().await?.get_peers_info_response.safe_get()?.peer_info.len(), 2);
    let tx_sub = TransactionSubmitter::default(pc, vec![]);
    tx_sub.with_faucet().await.expect("");

    let res = tx_sub.submit().await.expect("");
    assert!(res.query_transaction_response.is_some());

    Ok(())


    //
    // // TODO: Change the runtime structure to implement the shutdowns directly inside, then pass
    // // around the Arc reference externally for spawning.
    // let rt = build_simple_runtime(1, "test_run_main");
    // let args = RgArgs::default();
    // rt.spawn_blocking(move || main_from_args(args));
    // sleep(Duration::from_secs(10));
    // rt.shutdown_timeout(Duration::from_secs(10));
}