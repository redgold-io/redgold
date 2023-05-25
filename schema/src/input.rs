use crate::{SafeBytesAccess, SafeOption};
use crate::structs::{Address, ErrorInfo, Hash, Input, Output, Proof};

impl Input {
    pub fn verify_proof(&self, output: &Output, transaction_hash: &Hash) -> Result<(), ErrorInfo> {
        let address = output.address.safe_get()?;
        Proof::verify_proofs(&self.proof, &transaction_hash, address)
    }
    pub fn address(&self) -> Result<Address, ErrorInfo> {
        Proof::proofs_to_address(&self.proof)
    }
}