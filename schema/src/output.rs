use crate::structs::{ErrorInfo, Output, UtxoEntry};
use crate::transaction::amount_data;
use crate::{Address, HashClear, SafeOption};
use bitcoin::secp256k1::PublicKey;

pub fn output_data(address: Vec<u8>, amount: u64) -> Output {
    Output {
        address: Address::address_data(address),
        product_id: None,
        counter_party_proofs: vec![],
        data: amount_data(crate::transaction::amount_to_raw_amount(amount)),
        contract: None,
    }
}

pub fn tx_output_data(address: Address, amount: u64) -> Output {
    Output {
        address: Some(address),
        product_id: None,
        counter_party_proofs: vec![],
        data: amount_data(crate::transaction::amount_to_raw_amount(amount)),
        contract: None,
    }
}

impl HashClear for Output {
    fn hash_clear(&mut self) {}
}

impl Output {
    pub fn from_public_amount(public: &PublicKey, amount: u64) -> Output {
        Output {
            address: Address::address_data(Address::address(public).to_vec()),
            data: amount_data(amount),
            product_id: None,
            counter_party_proofs: vec![],
            contract: None,
        }
    }

    pub fn to_utxo_entry(
        &self,
        transaction_hash: &Vec<u8>,
        output_index: u32,
        time: u64,
    ) -> UtxoEntry {
        return UtxoEntry::from_output(self, transaction_hash, output_index as i64, time as i64);
    }

    pub fn amount(&self) -> u64 {
        self.data.as_ref().unwrap().amount.unwrap() as u64
    }

    pub fn safe_ensure_amount(&self) -> Result<&i64, ErrorInfo> {
        self.data.safe_get_msg("Missing data field on output")?
            .amount.safe_get_msg("Missing amount field on output")
    }

    pub fn opt_amount(&self) -> Option<i64> {
        self.data.safe_get_msg("Missing data field on output").ok().and_then(|data| data.amount)
    }

    pub fn rounded_amount(&self) -> f64 {
        crate::transaction::rounded_balance(self.amount())
    }
}
