/*
   let mut echo_hello = Command::new("md5sum");
   echo_hello.arg(path_str.clone());
   let hello_1 = echo_hello.output().expect("Ouput from command failure");
   let string1 = String::from_utf8(hello_1.stdout).expect("String decode failure");
   let md5stdout: String = string1
       .split_whitespace()
       .next()
       .expect("first output")
       .clone()
       .to_string();

   info!("Md5sum stdout from shell script: {}", md5stdout);

*/

// TODO: async version
// this doesn't need to scale as we're using it sparingly for ops and configs.

use std::process::Command;

pub fn run_cmd<S: Into<String>>(cmd: S, args: Vec<S>) -> (String, String) {
    let mut echo_hello = Command::new(cmd.into());
    for arg in args {
        echo_hello.arg(arg.into());
    }
    let hello_1 = echo_hello.output().expect("Ouput from command failure");
    let string1 = String::from_utf8(hello_1.stdout).expect("String decode failure");
    let string2 = String::from_utf8(hello_1.stderr).expect("String decode failure");
    (string1, string2)
}
