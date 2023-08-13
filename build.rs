
use std::process::Command;

fn main() {
    println!("Build script started");
    let o = Command::new("bash")
        .args("./sdk/compile.sh")
        .output()
        .expect("SDK compile failure");
    // println!("Build sdk compile output stdout: {}", )
    assert!(o.status.success());
}
