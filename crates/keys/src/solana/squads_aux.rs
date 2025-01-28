use crate::solana::multisig::MultisigProposeOutput;
use crate::solana::wallet::SolanaNetwork;
use metrics::counter;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::{Address, CurrencyAmount};
use redgold_schema::RgResult;
use solana_program::pubkey::Pubkey;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use std::str::FromStr;

impl SolanaNetwork {

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
    pub async fn multisig_propose_send_vault_create_tx(
        &self,
        multisig_pubkey: impl Into<String>,
        vault_index: Option<i64>,
        destination: Address,
        amount: CurrencyAmount,
        memo: Option<String>,
        priority_fee: Option<u64>
    ) -> RgResult<MultisigProposeOutput> {
        let multisig_pubkey = multisig_pubkey.into();
        counter!("redgold_multisig_solana_propose").increment(1);

        let vault_address = self.get_squads_vault_address(multisig_pubkey.clone(), vault_index.unwrap_or(0) as u8).await?;
        let vault_pubkey = Pubkey::from_str(&vault_address).unwrap();
        let destination_pubkey = Pubkey::from_str(&destination.render_string()?).unwrap();
        // Transfer from vault address
        let transfer_ix = solana_program::system_instruction::transfer(
            &vault_pubkey,  // from vault address
            &destination_pubkey,
            amount.amount as u64
        );

        // Include compute budget
        let instructions = vec![
            ComputeBudgetInstruction::set_compute_unit_price(priority_fee.unwrap_or(5000)),
            transfer_ix
        ];
        let blockhash = self.rpc_confirmed().await
            .get_latest_blockhash()
            .await
            .expect("Failed to get blockhash");


        // Use versioned message format
        let message = solana_sdk::message::v0::Message::try_compile(
            &self.self_pubkey()?,
            &instructions,
            &[],
            blockhash
        ).unwrap();

        let message_bytes = message.serialize();

        let vault_index = vault_index.unwrap_or(0);
        let memo_str = memo.map(|m| format!("--memo {}", m)).unwrap_or_default();
        let init = "vault-transaction-create";
        let remainder = format!(
            "--multisig-pubkey {} \
            --vault-index {} \
            --transaction-message {} \
            {}",
            multisig_pubkey,
            vault_index,
            hex::encode(message_bytes),
            // message_bytes.iter().map(|b| format!("--transaction-message {}", b.to_string())).join(" "),
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
            // tx_hash,
            transaction_index: tx_idx,
            vault_index
        };
        Ok(ret)
    }
}