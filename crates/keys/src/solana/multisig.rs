use std::env::home_dir;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::str::FromStr;
use itertools::Itertools;
use metrics::counter;
use serde::{Deserialize, Serialize};
use solana_program::message::Message;
use solana_sdk::pubkey::Pubkey;
use redgold_common_no_wasm::cmd::run_bash_async;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::{ErrorInfoContext, RgResult, SafeOption};
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::{Address, CurrencyAmount, NetworkEnvironment, SupportedCurrency};
use crate::solana::derive_solana::SolanaWordPassExt;
use crate::solana::wallet::SolanaNetwork;
use crate::TestConstants;
use crate::util::mnemonic_support::MnemonicSupport;

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
        std::fs::set_permissions("temp.exp", std::fs::Permissions::from_mode(0o755)).error_info("chmod")?;

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
        let tx_idx = vault_split.get(0).cloned().ok_msg("Transaction index not found")?.replace(" ", "");
        let tx_idx = i64::from_str(&tx_idx).unwrap();
        let latter_part = split.get(1).cloned().ok_msg("Multisig pubkey not found")?;
        let split = latter_part.split("Signature:").collect_vec();
        let txid = split.get(0).ok_msg("Multisig pubkey not found")?;
        Ok((txid.replace(" ", ""), tx_idx))
    }


    pub async fn keypair_json_bytes(&self) -> RgResult<String> {
        let (signing, verifying) = self.keys()?;
        let mut vec = signing.to_bytes().to_vec();
        vec.extend(verifying.to_bytes().to_vec());
        let s = vec.json_or();
        Ok(s)
    }

    //
    // // what do i do here?
    // pub async fn multisig_propose_send(&self,
    //                                    party_addrs_incl_self: Vec<Address>,
    //                                    destination: Address,
    //                                    amount: CurrencyAmount
    // ) -> RgResult<()> {
    //     let self_addr = self.self_address()?;
    //     Ok(())
    // }

    // Create a vault transaction to send funds
    pub async fn multisig_propose_send(
        &self,
        multisig_pubkey: impl Into<String>,
        vault_index: Option<i64>,
        destination: Address,
        amount: CurrencyAmount,
        memo: Option<String>
    ) -> RgResult<MultisigProposeOutput> {
        let multisig_pubkey = multisig_pubkey.into();
        counter!("redgold_multisig_solana_propose").increment(1);


        // Create transfer instruction
        let msig_pubkey = Pubkey::from_str(&*multisig_pubkey).unwrap();
        let transfer_ix = solana_program::system_instruction::transfer(
            &msig_pubkey,  // from
            &Pubkey::from_str(&destination.render_string()?).unwrap(),  // to
            amount.amount as u64
        );

        // Create message
        let message = Message::new(
            &[transfer_ix],
            Some(&msig_pubkey),  // payer
        );
        let message_bytes = message.serialize();

        let vault_index = vault_index.unwrap_or(0);
        let memo_str = memo.map(|m| format!("--memo {}", m)).unwrap_or_default();
        let init = "vault-transaction-create";
        let remainder = format!(
            "--multisig-pubkey {} \
            --vault-index {} \
            {} \
            {}",
            multisig_pubkey,
            vault_index,
            message_bytes.iter().map(|b| format!("--transaction-message {}", b.to_string())).join(" "),
            memo_str
        );

        // Transaction confirmed: 4sAZGBngmGqCMuPKVeY96UzgwDXR1rGeWvRjUuN999Z3jUf8deKZqyepM7R6Bro4UR2fHA4vDahnrmeiBXRJB98n
        // match on this.
        let (stdout, stderr) = self.cmd(init, remainder).await?;
        println!("propose send stdout: {}", stdout);
        println!("propose send stderr: {}", stderr);
        let (tx_hash, tx_idx) = Self::extract_multisig_send_stdout_txhash(stdout.clone())
            .with_detail("stdout", stdout)
            .with_detail("stderr", stderr)?;

        let ret = MultisigProposeOutput {
            multisig_pubkey,
            tx_hash,
            transaction_index: tx_idx,
            vault_index
        };
        Ok(ret)
    }

    // Vote/approve a transaction
    pub async fn multisig_approve_transaction(
        &self,
        multisig_pubkey: impl Into<String>,
        transaction_index: Option<u64>,
    ) -> RgResult<(String, String)> {
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

        self.cmd(init, remainder).await
    }

    // Execute the approved transaction
    pub async fn multisig_execute_transaction(
        &self,
        multisig_pubkey: impl Into<String>,
        transaction_index: Option<i64>,
    ) -> RgResult<(String, String)> {
        let init = "vault_transaction_execute";
        let remainder = format!(
            "--multisig-pubkey {} \
            --transaction-index {}",
            multisig_pubkey.into(),
            transaction_index.unwrap_or(0)
        );

        self.cmd(init, remainder).await
    }

}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct MultisigProposeOutput {
    pub multisig_pubkey: String,
    pub tx_hash: String,
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

#[ignore]
#[tokio::test]
async fn debug_kg() {
    let tc = TestConstants::new();
    let wp = tc.words_pass;
    let ci = TestConstants::test_words_pass().unwrap();
    let ci1 = ci.hash_derive_words("1").unwrap();
    let ci2 = ci.hash_derive_words("2").unwrap();
    //
    // let amount = 1_000_000; // 0.001 SOL
    // let amount = CurrencyAmount::from_currency(amount, SupportedCurrency::Solana);
    // let amount = CurrencyAmount::from_fractional_cur(0.99, SupportedCurrency::Solana).unwrap();

    let w = SolanaNetwork::new(NetworkEnvironment::Dev, Some(ci));
    let w1 = SolanaNetwork::new(NetworkEnvironment::Dev, Some(ci1));
    let w2 = SolanaNetwork::new(NetworkEnvironment::Dev, Some(ci2));

    println!("Wallet 1 address: {}", w.self_address().unwrap().render_string().unwrap());
    println!("Wallet 1 balance: {}", w.get_self_balance().await.unwrap().to_fractional());
    let party_addrs = vec![w.self_address().unwrap(), w1.self_address().unwrap(), w2.self_address().unwrap()];
    let threshold = 2;
    // let multisig_pubkey = w.establish_multisig_party(party_addrs, threshold).await.unwrap();
    let multisig_pubkey = "SSUXdtd957gaBMUA6aqEgBtByzKJ1mCQj7PC6Vqr8o7";
    println!("Multisig pubkey: {}", multisig_pubkey);
    // let res = w.send(Address::from_solana_external(&multisig_pubkey.to_string()),
    //         CurrencyAmount::from_fractional_cur(0.1, SupportedCurrency::Solana).unwrap()
    //         , None, None).await.unwrap();
    // println!("Sent: {}", res.message.hash().to_string());
    let destination = w1.self_address().unwrap();
    let res = w.multisig_propose_send(
        multisig_pubkey, Some(0),
        destination,  CurrencyAmount::from_fractional_cur(0.01, SupportedCurrency::Solana).unwrap(), None
    ).await.unwrap();
    println!("Proposed: {:?}", res);
    // println!("Proposed stderr: {}", res.1);

    /*
⠤ Sending transaction...
             Transaction confirmed:
              4sAZGBngmGqCMuPKVeY96UzgwDXR1rGeWvRjUuN999Z3jUf8deKZqyepM7R6Bro4UR2fHA4vDahnrmeiBXRJB98n
     */
    // w.init_multisig().await.unwrap();
    //
    // let approve1 = w.multisig_approve_transaction(multisig_pubkey, Some(0)).await.unwrap();
    // println!("Approve1: {}", approve1.0);
    // println!("Approve1 stderr: {}", approve1.1);


}