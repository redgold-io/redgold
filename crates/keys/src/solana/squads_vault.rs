use std::str::FromStr;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use solana_program::instruction::AccountMeta;
use redgold_schema::{ErrorInfoContext, RgResult, SafeOption};
use redgold_schema::errors::into_error::ToErrorInfo;
use redgold_schema::observability::errors::{EnhanceErrorInfo, Loggable};
use redgold_schema::structs::{Address, CurrencyAmount, SupportedCurrency};
use crate::solana::wallet::SolanaNetwork;

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct VaultTransaction {
    pub multisig: Pubkey,
    pub creator: Pubkey,
    pub index: u64,
    pub bump: u8,
    pub vault_index: u8,
    pub vault_bump: u8,
    pub ephemeral_signer_bumps: Vec<u8>,
    pub message: VaultTransactionMessage,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct VaultTransactionMessage {
    pub num_signers: u8,
    pub num_writable_signers: u8,
    pub num_writable_non_signers: u8,
    pub account_keys: Vec<Pubkey>,
    pub instructions: Vec<MultisigCompiledInstruction>,
    pub address_table_lookups: Vec<MultisigMessageAddressTableLookup>,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct MultisigCompiledInstruction {
    pub program_id_index: u8,
    pub account_indexes: Vec<u8>,
    pub data: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct MultisigMessageAddressTableLookup {
    pub account_key: Pubkey,
    pub writable_indexes: Vec<u8>,
    pub readonly_indexes: Vec<u8>,
}

impl VaultTransaction {
    pub fn try_deserialize(mut data: &[u8]) -> Result<Self, std::io::Error> {
        // Skip the 8-byte discriminator
        if data.len() < 8 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Data too short for discriminator",
            ));
        }
        data = &data[8..];

        Self::deserialize(&mut data)
    }

    pub fn try_deserialize_unchecked(data: &[u8]) -> Result<Self, std::io::Error> {
        Self::deserialize(&mut &data[..])
    }
}

impl VaultTransactionMessage {
    pub fn num_all_account_keys(&self) -> usize {
        let num_account_keys_from_lookups = self
            .address_table_lookups
            .iter()
            .map(|lookup| lookup.writable_indexes.len() + lookup.readonly_indexes.len())
            .sum::<usize>();

        self.account_keys.len() + num_account_keys_from_lookups
    }

    pub fn is_static_writable_index(&self, key_index: usize) -> bool {
        let num_account_keys = self.account_keys.len();
        let num_signers = usize::from(self.num_signers);
        let num_writable_signers = usize::from(self.num_writable_signers);
        let num_writable_non_signers = usize::from(self.num_writable_non_signers);

        if key_index >= num_account_keys {
            return false;
        }

        if key_index < num_writable_signers {
            return true;
        }

        if key_index >= num_signers {
            let index_into_non_signers = key_index.saturating_sub(num_signers);
            return index_into_non_signers < num_writable_non_signers;
        }

        false
    }

    pub fn is_signer_index(&self, key_index: usize) -> bool {
        key_index < usize::from(self.num_signers)
    }
}

// Example usage:
fn deserialize_account(data: &[u8]) -> Result<VaultTransaction, std::io::Error> {
    VaultTransaction::try_deserialize(data)
}


impl SolanaNetwork {

    pub async fn get_account_as_vault_tx(&self, a: impl Into<String>) -> RgResult<VaultTransaction> {
        let client = self.rpc_confirmed().await;
        let tx_address = a.into();
        let account = client.get_account(
            &Pubkey::from_str(&tx_address).error_info("pubkey")?
        ).await
            .error_info("get_account")?;

        let v = VaultTransaction
        ::try_deserialize(&account.data).error_info("deserialize")
            .with_detail("account_data", hex::encode(account.data))?;
        Ok(v)
    }

    pub async fn get_vault_tx_by_index(&self, multisig_address: impl Into<String>, index: u64) -> RgResult<VaultTransaction> {
        let tx_address = self.get_transaction_pda(multisig_address, index).await?;
        self.get_account_as_vault_tx(tx_address).await
    }
    pub async fn list_multisig_transactions_vault(&self, multisig_address: impl Into<String>) -> RgResult<Vec<VaultTransaction>> {
        let mut transactions = vec![];
        let mut index = 1u64;

        let string = multisig_address.into();
        loop {
            let tx_address = self.get_transaction_pda(string.clone(), index).await?;
            match self.get_account_as_vault_tx(tx_address).await.log_error() {
                Ok(transaction) => {
                    transactions.push(transaction);
                    index += 1;
                }
                Err(e) => {
                    // println!("Failed to get transaction {}: tx_address: {} : {}", index, tx_address, e);
                    break
                }
            }
        }

        Ok(transactions)
    }

}

impl VaultTransaction {
    pub fn decode_transfer(&self) -> RgResult<SolanaTransfer> {
        let instruction = self.message.instructions.first().ok_msg("Missing first instruction")?;

        // System program transfer instruction has index 2
        if instruction.data[0] != 2 {
            return "Unable to decode transfer".to_error();
        }

        // Get the from and to account indexes
        let from_index = instruction.account_indexes.get(0).ok_msg("Missing account index 0")?.clone() as usize;
        let to_index = instruction.account_indexes.get(1).ok_msg("Missing account index 1")?.clone() as usize;

        // Get the actual pubkeys
        let from = self.message.account_keys.get(from_index).ok_msg("Missing from key")?;
        let to = self.message.account_keys.get(to_index).ok_msg("Missing to key")?;

        // Amount is encoded as a little-endian u64 in the remaining data bytes
        let amount = u64::from_le_bytes(instruction.data[1..9].try_into().ok().ok_msg("Missing amount")?);
        let amount = CurrencyAmount::from_currency(amount as i64, SupportedCurrency::Solana);

        let t = SolanaTransfer {
            from: Address::from_solana_external(&from.to_string()),
            to: Address::from_solana_external(&to.to_string()),
            amount,
        };
        Ok(t)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Default, Clone)]
pub struct SolanaTransfer {
    pub from: Address,
    pub to: Address,
    pub amount: CurrencyAmount
}