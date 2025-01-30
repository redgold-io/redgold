use redgold_common_no_wasm::cmd::run_bash_async;
use redgold_schema::structs::{Address, NetworkEnvironment};
use redgold_schema::{structs, ErrorInfoContext, RgResult, SafeOption};
use serde::{Deserialize, Serialize};

// cast bind --crate-name safe-bindings keys/src/eth/safe.json

pub struct SafeMultisig {
    pub network: NetworkEnvironment,
    pub self_address: Address,
    pub private_hex: String
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct SafeCreationInfo {
    pub tx_hash: String,
    pub safe_addr: String
}


impl SafeMultisig {

    pub fn factory_address(&self) -> structs::Address {
        if self.network == NetworkEnvironment::Main {
            panic!("Main network not supported");
        } else {
            Address::from_eth_external_exact("0x4e1DCf7AD4e460CfD30791CCC4F9c8a4f820ec67")
        }
    }

    pub fn new(network: NetworkEnvironment, self_address: Address, private_hex: String) -> Self {
        Self {
            network,
            self_address,
            private_hex
        }
    }

    /**
     * usage:
        safe-creator [-h] [-v] [--threshold THRESHOLD] [--owners OWNERS [OWNERS ...]]
        [--safe-contract SAFE_CONTRACT] [--proxy-factory PROXY_FACTORY]
        [--callback-handler CALLBACK_HANDLER] [--salt-nonce SALT_NONCE] [--without-events] node_url private_key

        Example:
            safe-creator https://sepolia.drpc.org 0000000000000000000000000000000000000000000000000000000000000000


    positional arguments:
      node_url              Ethereum node url
      private_key           Deployer private_key

    options:
      -h, --help            show this help message and exit
      -v, --version         Show program's version number and exit.
      --threshold THRESHOLD
                            Number of owners required to execute transactions on the created Safe. It mustbe greater than 0 and less or equal than the number of owners
      --owners OWNERS [OWNERS ...]
                            Owners. By default it will be just the deployer
      --safe-contract SAFE_CONTRACT
                            Use a custom Safe master copy
      --proxy-factory PROXY_FACTORY
                            Use a custom proxy factory
      --callback-handler CALLBACK_HANDLER
                            Use a custom fallback handler. It is not required for Safe Master Copies with version < 1.1.0
      --salt-nonce SALT_NONCE
                            Use a custom nonce for the deployment. Same nonce with same deployment configuration will lead to the same Safe address
      --without-events      Use non events deployment of the Safe instead of the regular one. Recommended for mainnet to save gas costs when using the Safe
     */
    pub async fn create_safe(&self, threshold: i64, owners: Vec<Address>) -> RgResult<SafeCreationInfo> {
        let threshold = threshold.to_string();
        let owners_str = owners
            .iter().map(|x| x.render_string())
            .collect::<RgResult<Vec<String>>>()?;
        let owners_str = owners_str.join(" ");
        let rpc = self.rpc_url().await?;
        let private_key = self.private_hex.clone();
        let cmd = format!("docker run -it safeglobal/safe-cli safe-creator \
            --owners {owners_str} --threshold {threshold} {rpc} {private_key}");
        println!("cmd: {}", cmd);


        // Create expect script
        let expect_script = format!(
            r#"#!/usr/bin/expect -f
set timeout 300
spawn {cmd}
expect {{
    timeout {{ puts "Timeout waiting for initial prompt"; exit 1 }}
    "*continue*" {{ send "y\r" }}
}}
expect {{
    timeout {{ puts "Timeout waiting for deployment confirmation"; exit 1 }}
    "*Safe will be deployed*" {{ send "y\r" }}
}}
expect {{
    timeout {{ puts "Timeout waiting for transaction hash"; exit 1 }}
    "*tx-hash*" {{ puts "Transaction submitted successfully" }}
}}
expect eof"#
        );

        println!("Writing expect script: {}", expect_script.clone());

        std::fs::write("temp.exp", expect_script).error_info("write expect script")?;
        // std::fs::set_permissions("temp.exp", std::fs::Permissions::from_mode(0o755)).error_info("chmod")?;

        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions("temp.exp", std::fs::Permissions::from_mode(0o755)).error_info("chmod")?;
        }

        #[cfg(windows)]
        {
            let mut perms = std::fs::metadata("temp.exp").error_info("get metadata")?.permissions();
            perms.set_readonly(false);
            std::fs::set_permissions("temp.exp", perms).error_info("set permissions")?;
        }

        let cmd = "./temp.exp";
        let (stdout, stderr) = run_bash_async(
            cmd
        ).await?;
        println!("stdout: {}", stdout);
        println!("stderr: {}", stderr);
        Self::extract_creation_info(&stdout)
    }

    pub fn extract_creation_info(stdout: &str) -> RgResult<SafeCreationInfo> {
        let tx_hash_re = regex::Regex::new(r"tx-hash=(\w+)").error_info("Invalid regex")?;
        let safe_re = regex::Regex::new(r"Safe=(\w+)").error_info("Invalid regex")?;

        let tx_hash = tx_hash_re
            .captures(stdout)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
            .ok_msg("Could not find transaction hash in output")?;

        let safe_addr = safe_re
            .captures(stdout)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
            .ok_msg("Could not find Safe address in output")?;
        let info = SafeCreationInfo{
            tx_hash,
            safe_addr,
        };
        Ok(info)
    }
    /*
    this will get regexable information
     ____           __          ____  _      ___
/ ___|   __ _  / _|  ___   / ___|| |    |_ _|
\___ \  / _` || |_  / _ \ | |    | |     | |
 ___) || (_| ||  _||  __/ | |___ | |___  | |
|____/  \__,_||_|   \___|  \____||_____||___|


Version: 1.4.0
Warning: Input is not a terminal (fd=0).
Loading Safe information...
Address=0x449F629b6bf816db771b69388E5b02b30ED86ACe
Nonce=0
Threshold=2
Owners=['0xA729F9430fc31Cda6173A0e81B55bBC92426f759', '0x0504B07Ea15D1bE5e8c49B7e4cfF43f26743f9Ac', '0x4F7d0E7F809C2C278FB5D3F558bc0F71017c4847']
Master_copy=0x29fcB43b46531BcA003ddC8FCB67FFE91900C762
Modules=[]
Fallback_handler=0xfd0732Dc9E303f09fCEf3a7388Ad10A83459Ec99
Guard=0x0000000000000000000000000000000000000000
Balance_ether=0
Version=1.4.1
Safe Tx Service=https://safe-transaction-sepolia.safe.global/api/v1/safes/0x449F629b6bf816db771b69388E5b02b30ED86ACe/transactions/
Etherscan=https://sepolia.etherscan.io/address/0x449F629b6bf816db771b69388E5b02b30ED86ACe
Ledger=Disabled  Optional ledger library is not installed, run pip install safe-cli[ledger]
Trezor=Disabled  Optional trezor library is not installed, run pip install safe-cli[trezor]
    docker run safeglobal/safe-cli safe-cli 0x449F629b6bf816db771b69388E5b02b30ED86ACe https://sepolia.drpc.org
    */
    pub async fn safe_cli(
        &self,
        checksummed_safe_address: String,
        ethereum_node_url: String,
    ) -> RgResult<()> {
        let (stdout, stderr) = run_bash_async(
            format!("docker run -it safeglobal/safe-cli safe-cli {checksummed_safe_address} {ethereum_node_url}")
        ).await?;
        println!("stdout: {}", stdout);
        println!("stderr: {}", stderr);
        Ok(())
    }

    pub async fn rpc_url(&self) -> RgResult<String> {
        let res = {
            if self.network == NetworkEnvironment::Main {
                panic!("Main network not supported");
            } else {
                "https://sepolia.drpc.org"
            }
        };
        Ok(res.to_string())
    }
}

