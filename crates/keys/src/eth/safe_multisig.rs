use redgold_common_no_wasm::cmd::run_bash_async;
use redgold_schema::keys::words_pass::WordsPass;
use redgold_schema::{ErrorInfoContext, RgResult};
use redgold_schema::structs::{Address, ErrorInfo, NetworkEnvironment};
use crate::address_external::ToEthereumAddress;
use crate::TestConstants;
use crate::util::mnemonic_support::MnemonicSupport;

pub struct SafeMultisig {
    network: NetworkEnvironment,
    pub wp: WordsPass,
}


impl SafeMultisig {

    pub fn new(wp: WordsPass, network: NetworkEnvironment) -> Self {
        Self {
            wp,
            network
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
    pub async fn create_safe(&self, threshold: i64, owners: Vec<Address>) -> RgResult<()> {
        let threshold = threshold.to_string();
        let owners_str = owners
            .iter().map(|x| x.render_string())
            .collect::<RgResult<Vec<String>>>()?;
        let owners_str = owners_str.join(" ");
        let rpc = self.rpc_url().await?;
        let private_key = self.get_pk_hex()?;
        let cmd = format!("docker run -it safeglobal/safe-cli safe-creator \
            --owners {owners_str} --threshold {threshold} {rpc} {private_key}");
        println!("cmd: {}", cmd);


        // Create expect script
        let expect_script = format!(
            r#"#!/usr/bin/expect -f
spawn bash -c "{}"
expect "Do you want to continue"
send "y\r"
expect "Transaction confirmed:"
expect eof"#,
            cmd,
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
        Ok(())
    }
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

    pub fn get_pk_hex(&self) -> Result<String, ErrorInfo> {
        let private_key = self.wp.default_kp()?.to_private_hex();
        Ok(private_key)
    }

    pub fn self_eth_addr(&self) -> RgResult<Address> {
        let eth_addr = self.wp.default_kp()?.public_key().to_ethereum_address_typed()?;
        Ok(eth_addr)
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

#[ignore]
#[tokio::test]
pub async fn test_safe_multisig() {
    let ci = TestConstants::test_words_pass().unwrap();
    let ci1 = ci.hash_derive_words("1").unwrap();
    let ci2 = ci.hash_derive_words("2").unwrap();

    let safe = SafeMultisig::new(ci, NetworkEnvironment::Dev);
    let safe1 = SafeMultisig::new(ci1, NetworkEnvironment::Dev);
    let safe2 = SafeMultisig::new(ci2, NetworkEnvironment::Dev);
    let addrs = vec![safe.self_eth_addr().unwrap(), safe1.self_eth_addr().unwrap(), safe2.self_eth_addr().unwrap()];
    let res = safe.create_safe(2, addrs).await.unwrap();
}