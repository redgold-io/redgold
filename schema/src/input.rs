use crate::SafeBytesAccess;
use crate::structs::{Address, ErrorInfo, Hash, Input, Output, Proof};

impl Input {
    pub fn verify_proof(&self, output: &Output, transaction_hash: &Hash) -> Result<(), ErrorInfo> {
        Proof::verify_proofs(&self.proof, &transaction_hash.safe_bytes()?, &output.address.safe_bytes()?)
    }
    pub fn address(&self) -> Result<Address, ErrorInfo> {
        Proof::proofs_to_address(&self.proof)
    }
}