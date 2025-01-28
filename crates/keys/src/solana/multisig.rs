use crate::solana::derive_solana::SolanaWordPassExt;
use crate::solana::wallet::SolanaNetwork;
use crate::util::mnemonic_support::MnemonicSupport;
use crate::TestConstants;
use itertools::Itertools;
use metrics::counter;
use redgold_common_no_wasm::cmd::run_bash_async;
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::{Address, CurrencyAmount, NetworkEnvironment, SupportedCurrency};
use redgold_schema::{ErrorInfoContext, RgResult, SafeOption};
use serde::{Deserialize, Serialize};
use solana_program::message::{Message, VersionedMessage};
#[cfg(unix)]
use solana_sdk::account::Account;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::pubkey::Pubkey;
use std::env::home_dir;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Serialize, Deserialize)]
struct MemberPermission {
    address: String,
    permissions: i64
}

impl SolanaNetwork {

    pub async fn cmd(&self, init: impl Into<String>, s: impl Into<String>) -> RgResult<(String, String)> {
        let rpc = self.network_rpc_url();
        let keypair = "keypair.json";
        let p = PathBuf::from_str(keypair).unwrap();;
        std::fs::write(p, self.keypair_json_bytes().await?).error_info("fs write")?;

        // Create expect script
        let expect_script = format!(
            r#"#!/usr/bin/expect -f
spawn bash -c "squads-multisig-cli {} --rpc-url {} --keypair {} {}"
expect "Do you want to proceed?"
send "y\r"
expect "Transaction confirmed:"
expect eof"#,
            init.into(),
            rpc,
            keypair,
            s.into()
        );

        println!("Writing expect script: {}", expect_script.clone());

        std::fs::write("temp.exp", expect_script).error_info("write expect script")?;
        // std::fs::set_permissions("temp.exp", std::fs::Permissions::from_mode(0o755)).error_info("chmod")?;

        #[cfg(unix)]
        {
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
        run_bash_async(cmd).await
    }
    /*
    multisig_create --rpc_url <RPC_URL> --program_id <PROGRAM_ID> --keypair <KEYPAIR_PATH> --config_authority <CONFIG_AUTHORITY> --members <MEMBER_1> <MEMBER_2> ... --threshold <THRESHOLD>
Parameters
--rpc_url <RPC_URL>: (Optional) The URL of the Solana RPC endpoint. Defaults to mainnet if not specified.

--program_id <PROGRAM_ID>: (Optional) The ID of the multisig program. Defaults to a standard ID if not specified.

--keypair <KEYPAIR_PATH>: Path to your keypair file.

--config_authority <CONFIG_AUTHORITY>: (Optional) Address of the Program Config Authority.

--members <MEMBER_...>: List of members' public keys, separated by spaces.

--threshold <THRESHOLD>: The threshold number of signatures required for executing multisig transactions.
 */
    pub async fn establish_multisig_party(&self, party_addrs_incl_self: Vec<Address>, threshold: i64
    ) -> RgResult<String> {

        counter!("redgold_multisig_solana_establish").increment(1);

        let init = "multisig-create";
        // multisig_create --keypair /path/to/keypair.json
        // --members "Member1PubKey,Permission1" "Member2PubKey,Permission2" --threshold 2
        let remainder = format!(
            "--members '{}' \
            --threshold {}",
            party_addrs_incl_self.iter()
                .flat_map(|a| a.render_string().ok())
                // .map(|a| format!("\"{},7\"", a))
                .map(|a| format!("{},7", a))
                // .map(|a| MemberPermission {
                //     address: a.clone(),
                //     permissions: 7,
                // })
                // .collect::<Vec<MemberPermission>>().json_or(),
                .collect::<Vec<String>>().join(" "),
            threshold
        );
        let (stdout, stderr) = self.cmd(init, remainder).await?;
        println!("multisig create stdout: {}", stdout.clone());
        println!("multisig create stderr: {}", stderr.clone());
        Self::extract_multisig_stdout(stdout.clone())
            .with_detail("stdout", stdout)
            .with_detail("stderr", stderr)
    }

    pub fn extract_multisig_stdout(stdout: String) -> RgResult<String> {
        let split = stdout.split("Created Multisig: ").collect_vec();
        let multisig_pubkey = split.get(1).ok_msg("Multisig pubkey not found")?;
        let split = multisig_pubkey.split(". Signature:").collect_vec();
        let multisig_pubkey = split.get(0).ok_msg("Multisig pubkey not found")?;
        Ok(multisig_pubkey.to_string())
    }

    pub fn extract_multisig_send_stdout_txhash(stdout: String) -> RgResult<(String, i64)> {
        let split = stdout.split("Transaction confirmed: ").collect_vec();
        let beginning = split.get(0).cloned().ok_msg("Missing before confirmed")?;
        let mut config_split = beginning.split("Transaction Index:").collect_vec();
        let tx_idx = config_split.get(1).cloned().ok_msg("Transaction index not found")?;
        let vault_split = tx_idx.split("Vault Index:").collect_vec();
        let tx_idx = vault_split.get(0).cloned().ok_msg("Transaction index not found")?
            .replace(" ", "").trim().to_string();

        // println!("Expected tx idx {}", tx_idx.clone());
        let tx_idx = i64::from_str(&tx_idx).unwrap();
        let latter_part = split.get(1).cloned().ok_msg("Multisig pubkey not found")?;
        let split = latter_part.split("Signature:").collect_vec();
        let txid = split.get(0).ok_msg("Multisig pubkey not found")?;
        Ok((txid.replace(" ", ""), tx_idx))
    }

    pub fn extract_multisig_send_stdout_tx_idx(stdout: String) -> RgResult<i64> {
        let split = stdout.split("Transaction confirmed: ").collect_vec();
        let beginning = split.get(0).cloned().ok_msg("Missing before confirmed")?;
        let mut config_split = beginning.split("Transaction Index:").collect_vec();
        let tx_idx = config_split.get(1).cloned().ok_msg("Transaction index not found")?;
        let vault_split = tx_idx.split("Vault Index:").collect_vec();
        let tx_idx = vault_split.get(0).cloned().ok_msg("Transaction index not found")?
            .replace(" ", "").trim().to_string();
        //
        // // println!("Expected tx idx {}", tx_idx.clone());
        let tx_idx = i64::from_str(&tx_idx).unwrap();
        // let latter_part = split.get(1).cloned().ok_msg("Multisig pubkey not found")?;
        // let split = latter_part.split("Signature:").collect_vec();
        // let txid = split.get(0).ok_msg("Multisig pubkey not found")?;
        // Ok((txid.replace(" ", ""), tx_idx))
        Ok(tx_idx)
    }


    pub async fn keypair_json_bytes(&self) -> RgResult<String> {
        let (signing, verifying) = self.keys()?;
        let mut vec = signing.to_bytes().to_vec();
        vec.extend(verifying.to_bytes().to_vec());
        let s = vec.json_or();
        Ok(s)
    }


    // Vote/approve a transaction
    pub async fn multisig_approve_transaction(
        &self,
        multisig_pubkey: impl Into<String>,
        transaction_index: Option<u64>,
    ) -> RgResult<()> {
        counter!("redgold_multisig_solana_vote").increment(1);
        let init = "proposal-vote";
        let transaction_index = transaction_index.unwrap_or(0);
        let remainder = format!(
            "--multisig-pubkey {} \
            --transaction-index {} \
            --action Approve",
            multisig_pubkey.into(),
            transaction_index
        );

        let (stdout, stderr) = self.cmd(init, remainder).await?;
        println!("approve stdout: {}", stdout.clone());
        println!("approve stderr: {}", stderr.clone());
        if stdout.contains("Casted Approve vote") {
            Ok(())
        } else {
            "Failed to cast vote".to_error()
                .with_detail("stdout", stdout)
                .with_detail("stderr", stderr)
        }
    }

    // Execute the approved transaction
    pub async fn multisig_execute_transaction(
        &self,
        multisig_pubkey: impl Into<String>,
        transaction_index: Option<i64>,
    ) -> RgResult<String> {
        let init = "vault-transaction-execute";
        let remainder = format!(
            "--multisig-pubkey {} \
            --transaction-index {}",
            multisig_pubkey.into(),
            transaction_index.unwrap_or(0)
        );

        let (stdout, stderr) = self.cmd(init, remainder).await?;
        println!("execute stdout: {}", stdout.clone());
        println!("execute stderr: {}", stderr.clone());
        Ok(stdout)
    }

}



#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct MultisigProposeOutput {
    pub multisig_pubkey: String,
    // pub tx_hash: String,
    pub transaction_index: i64,
    pub vault_index: i64
}

#[ignore]
#[tokio::test]
async fn dump_kp() {
    let tc = TestConstants::new();
    let wp = tc.words_pass;
    let ci = TestConstants::test_words_pass().unwrap();
    let b = SolanaNetwork::convert_to_solana_keypair_bytes(&ci.derive_solana_keys().unwrap().0).unwrap();
    let s = b.json_or();
    let h = home_dir().unwrap();
    let path = h.join(".config/solana/id.json");
    std::fs::write(path, s).unwrap();
}



impl SolanaNetwork {
    //
    fn squads_program_id(&self) -> Pubkey{
        Pubkey::from_str(match self.net {
            NetworkEnvironment::Main => {"SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf"},
            _ => {"SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf"}
        }).unwrap()
    }
    const SEED_PREFIX: &'static [u8] = b"multisig";
    const SEED_VAULT: &'static [u8] = b"vault";
    const SEED_TRANSACTION: &'static [u8] = b"transaction";
    const SEED_PROPOSAL: &'static [u8] = b"proposal";
    const SEED_MULTISIG: &'static [u8] = b"multisig";
    const SEED_EPHEMERAL_SIGNER: &'static [u8] = b"ephemeral_signer";

    pub async fn get_squads_vault_address(&self, multisig_address: impl Into<String>, vault_index: u8) -> RgResult<String> {
        let program_id = self.squads_program_id();
        let multisig_pubkey = Pubkey::from_str(&*multisig_address.into())
            .error_info("Failed to parse multisig address")?;

        let (vault_address, _bump) = Pubkey::find_program_address(
            &[
                Self::SEED_PREFIX,
                multisig_pubkey.as_ref(),
                Self::SEED_VAULT,
                &[vault_index],
            ],
            &program_id,
        );

        Ok(vault_address.to_string())
    }

    pub async fn get_transaction_pda(&self, multisig_address: impl Into<String>, transaction_index: u64) -> RgResult<String> {
        let program_id = self.squads_program_id();
        let multisig_pubkey = Pubkey::from_str(&*multisig_address.into())
            .error_info("Failed to parse multisig address")?;

        let (tx_address, _bump) = Pubkey::find_program_address(
            &[
                Self::SEED_PREFIX,
                multisig_pubkey.as_ref(),
                Self::SEED_TRANSACTION,
                &transaction_index.to_le_bytes(),
            ],
            &program_id,
        );

        Ok(tx_address.to_string())
    }

    pub async fn get_proposal_pda(&self, multisig_address: impl Into<String>, transaction_index: u64) -> RgResult<String> {
        let program_id = self.squads_program_id();
        let multisig_pubkey = Pubkey::from_str(&*multisig_address.into())
            .error_info("Failed to parse multisig address")?;

        let (proposal_address, _bump) = Pubkey::find_program_address(
            &[
                Self::SEED_PREFIX,
                multisig_pubkey.as_ref(),
                Self::SEED_TRANSACTION,
                &transaction_index.to_le_bytes(),
                Self::SEED_PROPOSAL,
            ],
            &program_id,
        );

        Ok(proposal_address.to_string())
    }

    pub async fn get_ephemeral_signer_pda(&self, transaction_address: impl Into<String>, signer_index: u8) -> RgResult<String> {
        let program_id = self.squads_program_id();
        let transaction_pubkey = Pubkey::from_str(&*transaction_address.into())
            .error_info("Failed to parse transaction address")?;

        let (signer_address, _bump) = Pubkey::find_program_address(
            &[
                Self::SEED_PREFIX,
                &transaction_pubkey.to_bytes(),
                Self::SEED_EPHEMERAL_SIGNER,
                &[signer_index],
            ],
            &program_id,
        );

        Ok(signer_address.to_string())
    }

    // Optional: Get full transaction info including status
    pub async fn get_transaction_info(&self, tx_address: impl Into<String>) -> RgResult<Vec<u8>> {
        let client = self.rpc_confirmed().await;
        let pubkey = Pubkey::from_str(&tx_address.into()).error_info("pubkey")?;
        let account = client.get_account(&pubkey).await
            .error_info("Failed to get transaction account")?;
        Ok(account.data)
    }
}


// TODO: Attempt mainnet / ui testing to see why execute fails.
#[ignore]
#[tokio::test]
async fn debug_kg() {
    let ci = TestConstants::test_words_pass().unwrap();
    let ci1 = ci.hash_derive_words("1").unwrap();
    let ci2 = ci.hash_derive_words("2").unwrap();

    let network = NetworkEnvironment::Main;

    let w = SolanaNetwork::new(network.clone(), Some(ci));
    let w1 = SolanaNetwork::new(network.clone(), Some(ci1));
    let w2 = SolanaNetwork::new(network.clone(), Some(ci2));

    println!("Wallet 1 address: {}", w.self_address().unwrap().render_string().unwrap());
    println!("Wallet 1 balance: {}", w.get_self_balance().await.unwrap().to_fractional());


    println!("Wallet 2 address: {}", w1.self_address().unwrap().render_string().unwrap());
    println!("Wallet 2 balance: {}", w1.get_self_balance().await.unwrap().to_fractional());


    println!("Wallet 3 address: {}", w2.self_address().unwrap().render_string().unwrap());
    println!("Wallet 3 balance: {}", w2.get_self_balance().await.unwrap().to_fractional());

    let multisig_account_main = "3VkpcmEAwU7pRAJJRcB2eynnt91SQwpx3paqZz1RerYh";
    let multisig_pubkey_main = multisig_account_main.to_string();
    let squad_vault_main = "BDqYrHiqhtj8yJ2F8aBn5VruHppP89QKbtUFArVFRQLs";
    let multisig_pubkey = multisig_pubkey_main.clone();

    let a = w.get_squads_vault_address(multisig_pubkey.clone(), 0).await.unwrap();
    println!("Vault address: {}", a);
    assert_eq!(a, squad_vault_main);

    for i in 0..10 {
        let res = w.get_vault_tx_by_index(multisig_pubkey.clone(), i).await;
        if let Ok(v) = res {
            let d = v.decode_transfer();
            println!("Tx by index: {:?} {:?}", i, d);
        }
    }


    for tx in w.list_multisig_transactions_vault(multisig_pubkey).await.unwrap() {
        println!("Tx by list: {:?}", tx.index);
    }

    //
    // let res = w.send(
    //     Address::from_solana_external(&a),
    //     CurrencyAmount::from_fractional_cur(0.1, SupportedCurrency::Solana).unwrap(),
    //     None,
    //     None
    // ).await.unwrap();
    // println!("Sent: {}", res.message.hash().to_string());
    //
    // println!("Multisig pubkey: {}", multisig_pubkey);
    //
    // let destination = w2.self_address().unwrap();

    // let multisig_pubkey = multisig_account_main;
    // let res = w.initiate_normal_transaction(
    //     multisig_pubkey.clone(),
    //     Some(0),
    //     destination,
    //     CurrencyAmount::from_fractional_cur(0.01, SupportedCurrency::Solana).unwrap(),
    //     None,
    // ).await.unwrap();
    //
    // println!("Proposed: {:?}", res);


}

#[ignore]
#[tokio::test]
async fn e2e_multisig_example() {
    let ci = TestConstants::test_words_pass().unwrap();
    let ci1 = ci.hash_derive_words("1").unwrap();
    let ci2 = ci.hash_derive_words("2").unwrap();
    let network = NetworkEnvironment::Dev;

    let w = SolanaNetwork::new(network.clone(), Some(ci));
    let w1 = SolanaNetwork::new(network.clone(), Some(ci1));
    let w2 = SolanaNetwork::new(network.clone(), Some(ci2));

    println!("Wallet 1 address: {}", w.self_address().unwrap().render_string().unwrap());
    println!("Wallet 1 balance: {}", w.get_self_balance().await.unwrap().to_fractional());

    println!("Wallet 2 address: {}", w1.self_address().unwrap().render_string().unwrap());
    println!("Wallet 2 balance: {}", w1.get_self_balance().await.unwrap().to_fractional());

    println!("Wallet 3 address: {}", w2.self_address().unwrap().render_string().unwrap());
    println!("Wallet 3 balance: {}", w2.get_self_balance().await.unwrap().to_fractional());

    let party_addrs = vec![
        w.self_address().unwrap(), w1.self_address().unwrap(), w2.self_address().unwrap(),
    ];

    let threshold = 2;
    let multisig_pubkey = w.establish_multisig_party(party_addrs, threshold).await.unwrap();
    let a = w.get_squads_vault_address(multisig_pubkey.clone(), 0).await.unwrap();
    println!("Vault address: {}", a);

    let res = w.send(
        Address::from_solana_external(&a),
        CurrencyAmount::from_fractional_cur(0.1, SupportedCurrency::Solana).unwrap(),
        None,
        None
    ).await.unwrap();
    println!("Sent: {}", res.message.hash().to_string());


    println!("Multisig pubkey: {}", multisig_pubkey);

    let destination = w2.self_address().unwrap();

    let res = w.initiate_normal_transaction(
        multisig_pubkey.clone(),
        Some(0),
        destination,
        CurrencyAmount::from_fractional_cur(0.01, SupportedCurrency::Solana).unwrap(),
        None,
    ).await.unwrap();

    println!("Proposed: {:?}", res);

    // w.get_account_as_vault_tx()

    let tx_idx = res.transaction_index;
    let approve1 = w.multisig_approve_transaction(multisig_pubkey.clone(), Some(tx_idx as u64)).await.unwrap();
    let approve2 = w1.multisig_approve_transaction(multisig_pubkey.clone(), Some(tx_idx as u64)).await.unwrap();
    let txid = w.multisig_execute_transaction(multisig_pubkey.clone(), Some(tx_idx)).await.unwrap();

    println!("Executed: {}", txid);


}