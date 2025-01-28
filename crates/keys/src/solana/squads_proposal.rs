use crate::solana::wallet::SolanaNetwork;
use borsh::{BorshDeserialize, BorshSerialize};
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::{ErrorInfoContext, RgResult};
use solana_program::pubkey::Pubkey;
use std::str::FromStr;


#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub enum ProposalStatus {
    Draft { timestamp: i64 },
    Active { timestamp: i64 },
    Rejected { timestamp: i64 },
    Approved { timestamp: i64 },
    #[deprecated(note = "No longer needed for reentrancy protection")]
    Executing,
    Executed { timestamp: i64 },
    Cancelled { timestamp: i64 }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct Proposal {
    pub multisig: Pubkey,
    pub transaction_index: u64,
    pub status: ProposalStatus,
    pub bump: u8,
    pub approved: Vec<Pubkey>,
    pub rejected: Vec<Pubkey>,
    pub cancelled: Vec<Pubkey>
}

impl Proposal {
    pub fn size(members_len: usize) -> usize {
        // No discriminator needed for Borsh
        32 + // multisig
            8 +  // index
            1 +  // status enum variant
            8 +  // status timestamp
            1 +  // bump
            (4 + (members_len * 32)) + // approved vec
            (4 + (members_len * 32)) + // rejected vec
            (4 + (members_len * 32))   // cancelled vec
    }

    pub fn try_deserialize(data: &[u8]) -> Result<Self, std::io::Error> {
        Self::try_from_slice(data)
    }
}
impl SolanaNetwork {

    pub async fn get_tx_proposal(&self, addr: impl Into<String>) -> RgResult<Proposal> {
        let index = 0;
        let client = self.rpc_confirmed().await;
        let tx_address = addr.into();
        match client.get_account(&Pubkey::from_str(&tx_address).error_info("pubkey")?).await {
            Ok(account) => {
                println!("Raw account data (first 32 bytes): {:?}", &account.data[..32]);
                println!("Account data length: {}", account.data.len());
                println!("Full account data: {:?}", &account.data);
                match Proposal::try_deserialize(&account.data[8..]) {
                    Ok(transaction) => {
                        return Ok(transaction);
                    },
                    Err(e) => {
                        println!("Failed to deserialize transaction {}: {}", index, e);
                    }
                }
            },
            Err(e) => {
                println!("Failed to get transaction {}: tx_address: {} : {}", index, tx_address, e);
            }
        }
        "Failed".to_error()
    }
    pub async fn list_multisig_transactions_vault_proposal(&self, multisig_address: impl Into<String>) -> RgResult<Vec<Proposal>> {
        let client = self.rpc_confirmed().await;
        let mut transactions = vec![];
        let mut index = 0u64;

        let string = multisig_address.into();
        loop {
            let tx_address = self.get_proposal_pda(string.clone(), index).await?;
            println!("Attempting query on tx address from proposal pda index {} {}", index, tx_address);

        }

        Ok(transactions)
    }
}