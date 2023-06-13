use crate::{SafeBytesAccess, SafeOption};
use crate::structs::{Address, ErrorInfo, Hash, Input, Output, Proof};

impl Input {
    pub fn verify_proof(&self, address: &Address, hash: &Hash) -> Result<(), ErrorInfo> {
        Proof::verify_proofs(&self.proof, &hash, address)
    }

    pub fn address(&self) -> Result<Address, ErrorInfo> {
        Proof::proofs_to_address(&self.proof)
    }

    // This does not verify the address on the prior output
    pub fn verify_signatures_only(&self, hash: &Hash) -> Result<(), ErrorInfo> {
        for proof in &self.proof {
            proof.verify(&hash)?
        }
        return Ok(());
    }

    pub fn verify(&self, hash: &Hash) -> Result<(), ErrorInfo> {
        let o = self.output.safe_get_msg("Missing enriched output on input for transaction verification")?;
        let address = o.address.safe_get_msg("Missing address on enriched output for transaction verification")?;
        self.verify_proof(&address, hash)?;
        return Ok(());
    }
}