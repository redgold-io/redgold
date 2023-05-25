use std::borrow::Borrow;
use std::thread::sleep;
use std::time::Duration;
use itertools::Itertools;
use redgold::api::control_api::ControlClient;
use redgold::api::public_api::PublicClient;
use redgold::canary::tx_gen::{SpendableUTXO, TransactionGenerator};
use redgold::canary::tx_submit::TransactionSubmitter;
use redgold::core::run_main::main_from_args;
use redgold::util;
use redgold::util::cli::args::RgArgs;
use redgold::util::runtimes::{build_runtime, build_simple_runtime};
use redgold_schema::SafeBytesAccess;
use redgold_schema::structs::NetworkEnvironment;
use redgold_schema::util::mnemonic_words::MnemonicWords;

#[test]
fn local_e2e_it() {

    util::init_logger().expect("log");
    let rt = build_runtime(1, "test");
    println!("Local E2E IT from inside test");

    let port_offset = NetworkEnvironment::Local.default_port_offset();
    let pc = PublicClient::from("127.0.0.1".to_string(), port_offset + 1);

    let mut tx_sub = TransactionSubmitter::default(pc, rt, vec![]);
    tx_sub.with_faucet();

    let res = tx_sub.submit();
    assert!(res.accepted());

    let cc = ControlClient::local(port_offset + 2);

    //
    // // TODO: Change the runtime structure to implement the shutdowns directly inside, then pass
    // // around the Arc reference externally for spawning.
    // let rt = build_simple_runtime(1, "test_run_main");
    // let args = RgArgs::default();
    // rt.spawn_blocking(move || main_from_args(args));
    // sleep(Duration::from_secs(10));
    // rt.shutdown_timeout(Duration::from_secs(10));
}