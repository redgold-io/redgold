use crate::solana::multisig::MultisigProposeOutput;
use crate::solana::wallet::SolanaNetwork;
use metrics::counter;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::{Address, CurrencyAmount};
use redgold_schema::RgResult;
use solana_program::pubkey::Pubkey;
use std::str::FromStr;

impl SolanaNetwork {
    pub async fn initiate_normal_transaction(
        &self,
        multisig_pubkey: impl Into<String>,
        vault_index: Option<i64>,
        destination: Address,
        amount: CurrencyAmount,
        memo: Option<String>
    ) -> RgResult<MultisigProposeOutput> {
        let multisig_pubkey = multisig_pubkey.into();
        counter!("redgold_multisig_solana_propose_transfer").increment(1);
        let destination_pubkey = destination.render_string()?;
        let vault_index = vault_index.unwrap_or(0);
        let memo_str = memo.map(|m| format!("--memo {}", m)).unwrap_or_default();
        let init = "initiate-normal-transfer";
        let remainder = format!(
            "--amount {} \
            --recipient {} \
            --multisig-pubkey {} \
            --vault-index {} \
            {}",
            amount.amount,
            destination_pubkey,
            multisig_pubkey,
            vault_index,
            memo_str
        );

        let (stdout, stderr) = self.cmd(init, remainder).await?;
        println!("propose send stdout: {}", stdout);
        println!("propose send stderr: {}", stderr);
        let (tx_idx) = Self::extract_multisig_send_stdout_tx_idx(stdout.clone())
            .with_detail("stdout", stdout)
            .with_detail("stderr", stderr)?;

        let ret = MultisigProposeOutput {
            multisig_pubkey,
            transaction_index: tx_idx,
            vault_index
        };
        Ok(ret)
    }

}