use std::env;
use std::process::Command;
use redgold_schema::{ErrorInfoContext, RgResult};
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::ErrorInfo;

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

pub async fn run_cmd_safe_async(cmd: impl Into<String>, args: Vec<impl Into<String>>) -> RgResult<(String, String)> {
    let program = cmd.into();
    let mut command = tokio::process::Command::new(program.clone());
    for arg in args {
        command.arg(arg.into());
    }
    let cmd_output = command.output().await.error_info("Ouput from command failure ")
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

pub fn run_powershell(cmd: impl Into<String>) -> RgResult<(String, String)> {
    run_cmd_safe("powershell", vec!["-Command", &cmd.into()])
}

pub async fn run_bash_async(cmd: impl Into<String>) -> RgResult<(String, String)> {
    run_cmd_safe_async("bash", vec!["-c", &cmd.into()]).await
}

pub async fn run_powershell_async(cmd: impl Into<String>) -> RgResult<(String, String)> {
    run_cmd_safe_async("powershell", vec!["-Command", &cmd.into()]).await
}

pub fn is_windows() -> bool {
    env::consts::OS == "windows"
}


pub async fn run_command_os(cmd: String) -> Result<(String, String), ErrorInfo> {
    Ok(if !is_windows() {
        run_bash_async(cmd).await?
    } else {
        run_powershell_async(cmd).await?
    })
}
