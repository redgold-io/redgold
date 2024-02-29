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
use itertools::Itertools;
use redgold_schema::{error_info, ErrorInfoContext, RgResult, SafeOption};
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
    let mut command = Command::new(program.clone());
    for arg in args {
        command.arg(arg.into());
    }
    let cmd_output = command.output().error_info("Ouput from command failure ")
        .add(program.clone())?;
    let stdout = String::from_utf8(cmd_output.stdout).error_info("stdout String decode failure ")
        .add(program.clone())?;
    let stderr = String::from_utf8(cmd_output.stderr).error_info("stderr String decode failure ")
        .add(program.clone())?;
    Ok((stdout, stderr))
}

pub fn run_bash(cmd: impl Into<String>) -> RgResult<(String, String)> {
    run_cmd_safe("bash", vec!["-c", &cmd.into()])
}

pub fn available_bytes(path: impl Into<String>, is_windows: bool) -> RgResult<i64> {
    if is_windows {
        return Err(error_info("available_bytes Not implemented for Windows"));
    }

    let out = run_bash(format!("df {}", path.into()))?.0;

    /*
    Mac:
    Filesystem   512-blocks       Used Available Capacity iused      ifree %iused  Mounted on
    /dev/disk3s5 1942700360 1341143528 552360152    71% 4990532 2761800760    0%   /System/Volumes/Data
    Linux:
    Filesystem     1K-blocks      Used Available Use% Mounted on
    /dev/sda3      456042808 160028204 272775436  37% /
     */
    let mut split = out.split("\n");
    let head = split.next().ok_msg("Missing stdout from df disk space command")?;

    // Use regex to split lines by one or more whitespace characters
    // let re = Regex::new(r"\s+").expect("Failed to compile regex");

    let blocks = head.split_whitespace().dropping(1).next().ok_msg("Missing second column from df disk space command")?
        .split("-").next().ok_msg("Missing first part of second column from df disk space command")?.to_lowercase();

    let multiplier: i64 = match blocks {
        "1k" => {1024}
        "512" => {512}
        _ => {
            return Err(error_info(format!("Unknown block size: {}", blocks)))
        }
    };
    let second = split.next().ok_msg("Missing second line from df disk space command")?;
    let available_bytes = second.split_whitespace().dropping(3).next()
        .ok_msg("Missing fourth column from df disk space command")?
        .parse::<i64>().error_info("Failed to parse available bytes")? * multiplier;
    Ok(available_bytes)

}

#[test]
fn test_disk() {
    let b = available_bytes("~", false).unwrap();
    let gb = b / (1024 * 1024 * 1024);
    println!("Available bytes in gb: {}", gb);
}

