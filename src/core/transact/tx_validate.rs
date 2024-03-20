use redgold_keys::tx_proof_validate::TransactionProofValidator;
use redgold_schema::{error_info, RgResult};
use redgold_schema::structs::{Address, NetworkEnvironment, Transaction};
use redgold_schema::fee_validator::TransactionFeeValidator;

pub trait TransactionValidator {
    fn validate(&self, fee_addrs: Option<&Vec<Address>>, network: Option<&NetworkEnvironment>) -> RgResult<()>;
}

impl TransactionValidator for Transaction {
    fn validate(&self, fee_addrs: Option<&Vec<Address>>, network: Option<&NetworkEnvironment>) -> RgResult<()> {
        self.validate_keys(network)?;
        if let Some(addrs) = fee_addrs {
            // Temporary bypass for node config updates, to be removed later
            let allow_bypass = self.outputs.iter().all(|o| o.is_metadata() || o.observation().is_ok());
            if !self.validate_fee(addrs) && !allow_bypass {
                return Err(error_info("Transaction fee is too low or to unsupported fee address"));
            };
        }
        Ok(())

    }
}
