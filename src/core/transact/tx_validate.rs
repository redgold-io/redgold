use redgold_keys::tx_proof_validate::TransactionProofValidator;
use redgold_schema::{EasyJson, error_info, RgResult};
use redgold_schema::errors::EnhanceErrorInfo;
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
            let allow_bypass = self.is_metadata_or_obs();
            if !self.validate_fee(addrs) && !allow_bypass {
                let result = Err(error_info("Transaction fee is too low or to unsupported fee address"))
                    .with_detail("transaction", self.json_or())
                    .with_detail("fee_addrs", fee_addrs.json_or());
                return result

            };
        }
        Ok(())

    }
}
