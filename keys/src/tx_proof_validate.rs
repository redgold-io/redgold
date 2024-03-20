use redgold_schema::errors::EnhanceErrorInfo;
use redgold_schema::{EasyJson, RgResult};
use redgold_schema::structs::Transaction;
use crate::transaction_support::InputSupport;

pub trait TransactionProofValidator {
    fn validate_signatures(&self) -> bool;
}

impl TransactionProofValidator for Transaction {
    fn validate_signatures(&self) -> RgResult<()> {
        validate_inner(&self).add("Validate signatures failed. Transaction:").add(self.json_or())
    }

}
fn validate_inner(tx: &Transaction) -> RgResult<()> {
    let hash = tx.signable_hash();
    for input in &tx.inputs {
        if let Ok(a) = input.address() {
            input.verify_proof(&a, &hash).add(input.json_or()).add(hash.hex())?;
        }
    }
    Ok(())
}