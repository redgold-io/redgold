use crate::{SafeBytesAccess, SafeOption};
use crate::structs::{Address, ErrorInfo, Hash, Input, Output, Proof};

impl Input {

    pub fn address(&self) -> Result<Address, ErrorInfo> {
        Proof::proofs_to_address(&self.proof)
    }

}