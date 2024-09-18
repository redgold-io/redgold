
use std::thread::sleep;
use std::time::Duration;
use redgold::core::run_main::main_from_args;
use redgold_schema::conf::rg_args::RgArgs;
use redgold::util::runtimes::{build_simple_runtime};

#[test]
fn run_main() {

    // TODO: Change the runtime structure to implement the shutdowns directly inside, then pass
    // around the Arc reference externally for spawning.
    let rt = build_simple_runtime(1, "test_run_main");
    let args = RgArgs::default();
    rt.spawn_blocking(move || main_from_args(args));
    sleep(Duration::from_secs(10));
    rt.shutdown_timeout(Duration::from_secs(10));
}