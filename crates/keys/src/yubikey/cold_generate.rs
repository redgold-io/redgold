use redgold_common_no_wasm::cmd::run_bash_async;
use redgold_schema::{keys::words_pass::WordsPass, RgResult};

use crate::util::mnemonic_support::MnemonicSupport;



pub async fn ykman_installed() -> bool {
    let cli_installed = run_bash_async("ykman").await;
    if let Ok((stdout, _stderr)) = cli_installed {
        if stdout.contains("Configure your YubiKey via the command line") {
            return true;
        }
    }
    false
}


/*
ykman config set-lock-code [OPTIONS]
Set or change the configuration lock code. The configuration lock code only applies to the management application. A lock code may be used to protect the application configuration. The lock code must be a 32 characters (16 bytes) hex value.

Once this code is set, if the user attempts to toggle the on/off state of any of the applications on the key, they are prompted for the configuration lock code. It is only toggling that triggers this; no such prompt appears if a user adds or removes an OATH-TOTP credential, for example.

This command was introduced with firmware version 5.0.

Options
Option	Description
-h, --help	Show this message and exit.
-c, --clear	Clear the lock code.
-f, --force	Confirm the action without prompting.
-g, --generate	
Generate a random lock code. Conflicts
with --new-lock-code.
-l, --lock-code HEX	Current lock code.
-n, --new-lock-code HEX	New lock code. Conflicts with --generate */

pub async fn set_lock_code(current: impl Into<String>, new_lock: impl Into<String>) {
    let cmd = format!("ykman config set-lock-code -l {} -n {}", current.into(), new_lock.into());
    let (stdout, stderr) = run_bash_async(&cmd).await.unwrap();
    println!("set_lock_code: {:?} {:?}", stdout, stderr);
}

pub async fn cold_generate(words: &WordsPass) -> RgResult<()> {

    let linux_download = "https://developers.yubico.com/yubikey-manager/Releases/yubikey_manager-5.5.0.tar.gz";
    let download_link = "https://www.yubico.com/support/download/yubikey-manager/";

    println!("cli_installed: {:?}", ykman_installed().await);

    Ok(())
}

#[tokio::test]
async fn test_cold_generate() {
    let words = WordsPass::test_words();
    let result = cold_generate(&words).await;
    result.unwrap();
}