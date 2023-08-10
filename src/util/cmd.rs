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
use redgold_schema::{ErrorInfoContext, RgResult};
use redgold_schema::errors::EnhanceErrorInfo;

pub fn run_cmd(cmd: impl Into<String>, args: Vec<impl Into<String>>) -> (String, String) {
    let mut echo_hello = Command::new(cmd.into());
    for arg in args {
        echo_hello.arg(arg.into());
    }
    let hello_1 = echo_hello.output().expect("Ouput from command failure");
    let string1 = String::from_utf8(hello_1.stdout).expect("String decode failure");
    let string2 = String::from_utf8(hello_1.stderr).expect("String decode failure");
    (string1, string2)
}

pub fn run_cmd_safe(cmd: impl Into<String>, args: Vec<impl Into<String>>) -> RgResult<(String, String)> {
    let program = cmd.into();
    let mut echo_hello = Command::new(program.clone());
    for arg in args {
        echo_hello.arg(arg.into());
    }
    let hello_1 = echo_hello.output().error_info("Ouput from command failure ")
        .add(program.clone())?;
    let string1 = String::from_utf8(hello_1.stdout).error_info("stdout String decode failure ")
        .add(program.clone())?;
    let string2 = String::from_utf8(hello_1.stderr).error_info("stderr String decode failure ")
        .add(program.clone())?;
    Ok((string1, string2))
}

pub fn run_bash(cmd: impl Into<String>) -> RgResult<(String, String)> {
    run_cmd_safe("bash", vec!["-c", &cmd.into()])
}