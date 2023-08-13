#![feature(exit_status_error)]

use std::process::Command;

fn main() {
    println!("Build script started");
    let o = Command::new("./sdk/compile.sh").output().expect("SDK compile failure");
    // println!("Build sdk compile output stdout: {}", )
    o.status.exit_ok().expect("okay");
}
