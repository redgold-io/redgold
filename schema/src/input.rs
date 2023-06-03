use crate::{SafeBytesAccess, SafeOption};
use crate::structs::{Address, ErrorInfo, Hash, Input, Output, Proof};

impl Input {
    pub fn verify_proof(&self, address: &Address, hash: &Hash) -> Result<(), ErrorInfo> {
        Proof::verify_proofs(&self.proof, &hash, address)
    }

    pub fn address(&self) -> Result<Address, ErrorInfo> {
        Proof::proofs_to_address(&self.proof)
    }
}