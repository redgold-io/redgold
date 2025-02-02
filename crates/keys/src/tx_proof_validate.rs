use crate::transaction_support::InputSupport;
use itertools::Itertools;
use log::info;
use redgold_schema::helpers::easy_json::EasyJson;
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::{AddressType, NetworkEnvironment, SupportedCurrency, Transaction};
use redgold_schema::tx_schema_validate::SchemaValidationSupport;
use redgold_schema::{error_info, RgResult, SafeOption};
use redgold_schema::proto_serde::ProtoSerde;

pub trait TransactionProofValidator {
    fn validate_signatures(&self) -> RgResult<()>;
    fn validate_keys(&self, network_opt: Option<&NetworkEnvironment>) -> RgResult<()>;
}

impl TransactionProofValidator for Transaction {
    fn validate_signatures(&self) -> RgResult<()> {
        validate_inner(&self).add("Validate signatures failed. Transaction:").add(self.json_or())
    }

    fn validate_keys(&self, network_opt: Option<&NetworkEnvironment>) -> RgResult<()> {
        self.validate_schema(network_opt, true)?;
        self.validate_signatures()?;
        Ok(())
    }
}
fn validate_inner(tx: &Transaction) -> RgResult<()> {
    let hash = tx.signable_hash();
    for input in &tx.inputs {
        // info!("Validate inner new func address: {}", input.address().unwrap().render_string().unwrap());
        // info!("Validate inner proof length: {}", input.proof.len());
        let pk = input.proof.get(0).cloned().unwrap().public_key.unwrap();
        // info!("Validate inner proof 0 pk: {}", pk.hex());
        // info!("Validate inner proof 0 pk address direct: {}", pk.address().unwrap().render_string().unwrap());
        if let Ok(a) = input.address() {
            input.verify_proof(&a, &hash).add(input.json_or()).add(hash.hex())?;
        }
    }
    // TODO: Validate deposit proofs.

    Ok(())
}

fn validate_deposit_addresses(tx: &Transaction) -> RgResult<()> {

    let res = tx.output_request()
        .flat_map(|r| r.stake_request.as_ref())
        .flat_map(|r| r.deposit.as_ref())
        .flat_map(|r| r.deposit.as_ref())
        .collect_vec();

    for d in res {
        let amt = d.amount.safe_get_msg("Missing amount in deposit request")?;
        if amt.currency_or() == SupportedCurrency::Redgold {
            return Err(error_info("Redgold deposit not allowed in external deposit request transaction"));
        }
        let addr = d.address.safe_get_msg("Missing address in deposit request")?;
        let allowed_external_deposit_addrs = vec![AddressType::EthereumExternalString, AddressType::BitcoinExternalString];
        let ato = AddressType::from_i32(addr.address_type);
        let at = ato.safe_get_msg("Missing address type in deposit request")?;
        if !allowed_external_deposit_addrs.contains(&at) {
            return Err(error_info("Invalid address type in deposit request"));
        }
    }
    Ok(())
}