use crate::constants::MAX_INPUTS_OUTPUTS;
use crate::helpers::easy_json::EasyJson;
use crate::helpers::with_metadata_hashable::WithMetadataHashable;
use crate::observability::errors::EnhanceErrorInfo;
use crate::pow::TransactionPowValidate;
use crate::proto_serde::ProtoSerde;
use crate::structs::{NetworkEnvironment, Transaction, UtxoId};
use crate::transaction::MAX_TRANSACTION_MESSAGE_SIZE;
use crate::util::times;
use crate::{error_code, error_info, error_message, structs, RgResult, SafeOption};
use std::collections::HashSet;

pub trait SchemaValidationSupport {
    fn validate_schema(&self, network_opt: Option<&NetworkEnvironment>, expect_signed: bool) -> RgResult<()>;
    fn validate_current_time(&self, max_delta: Option<i64>) -> RgResult<()>;
}

const DUST_LIMIT : i64 = 1000;
const MAX_TX_BYTE_SIZE: usize = 100_000;
impl SchemaValidationSupport for Transaction  {
    fn validate_schema(&self, network_opt: Option<&NetworkEnvironment>, expect_signed: bool) -> RgResult<()> {

        for output in self.outputs.iter() {
            if let Some(a) = output.opt_amount_typed() {
                if a.amount < DUST_LIMIT {
                    Err(error_info("Output amount is below dust limit for output")).add(output.json_or())?;
                }
            }
        }
        self.pow_validate()?;

        let size_bytes = self.proto_serialize().len();

        if size_bytes > MAX_TX_BYTE_SIZE {
            let mut info = error_info(format!("Transaction size: {} too large, expected {}", size_bytes, 10_000));
            info.with_detail("size_bytes", size_bytes.to_string());
            Err(info)?;
        }

        let options = self.options.safe_get_msg("Missing options on transaction")?;
        let network_i32 = options.network_type.safe_get_msg("Missing network type on transaction")?;
        let network = NetworkEnvironment::from_i32(network_i32.clone())
            .ok_msg("Invalid network type on transaction")?;

        if let Some(n) = network_opt {
            if n != &network {
                Err(error_info("Network type mismatch"))?;
            }
        }

        if options.contract.is_some() && network.is_main() {
            Err(error_info("Contract transactions not yet supported"))?
        }

        let mut hs: HashSet<UtxoId> = HashSet::new();
        for i in &self.inputs {
            if let Some(f) = &i.utxo_id {
                if hs.contains(f) {
                    return Err(error_info("Duplicate input UTXO consumption"))?;
                }
                hs.insert(f.clone());
            }
        }
        // TODO: Deal with this later for genesis / nmd
        if self.inputs.is_empty() {
            // if all nmd or
            if !self.is_metadata_or_obs() {
                Err(error_code(structs::ErrorCode::MissingInputs))?;
            }
        }
        if self.outputs.is_empty() {
            Err(error_code(structs::ErrorCode::MissingOutputs))?;
        }

        if let Some(o) = &self.options {
            if let Some(d) = &o.data {
                if let Some(m) = &d.message {
                    let i = m.len();
                    if i > MAX_TRANSACTION_MESSAGE_SIZE {
                        Err(
                            error_info(
                                format!(
                                    "Message length: {} too large, expected {}", i, MAX_TRANSACTION_MESSAGE_SIZE
                                )
                            )
                        )?;
                    }
                }
            }
        }
        for input in self.inputs.iter() {
            if let Some(utxo) = input.utxo_id.as_ref() {
                if utxo.output_index > (MAX_INPUTS_OUTPUTS as i64) {
                    Err(error_code(structs::ErrorCode::InvalidAddressInputIndex))?;
                }
            }
            if expect_signed {
                if input.proof.is_empty() {
                    let floating_non_consume_input = input.utxo_id.is_none() && input.floating_utxo_id.is_some();
                    if !floating_non_consume_input {
                        Err(error_message(structs::ErrorCode::MissingProof,
                                          format!("Input proof is missing on input {}", input.json_or()
                                          )))?;
                    }
                }
            }
        }

        for _output in self.outputs.iter() {
            // TODO: Reimplement dust separate from testing?
            // if output.address.len() != 20 {
            //     // println!("address id len : {:?}", output.address.len());
            //     return Err(RGError::InvalidHashLength);
            // }
            // if let Some(a) = _output.opt_amount() {
            //     if a < 10_000 {
            //         Err(error_info(format!("Insufficient amount output of {a}")))?;
            //     }
            // }
        }

        // TODO: Sum by product Id

        return Ok(());
    }

    fn validate_current_time(&self, max_delta: Option<i64>) -> RgResult<()> {
        let current_time = times::current_time_millis();
        let max_delta = max_delta.unwrap_or(1000 * 60 * 15); // 15 minutes delay
        let this_time = self.time()?.clone();
        if this_time > current_time {
            Err(error_info("Transaction timestamp is too far in the future"))?;
        }
        if current_time - this_time > max_delta {
            Err(error_info("Transaction timestamp is too far in the past"))
                .with_detail("current_time", current_time.to_string())
                .with_detail("this_time", this_time.to_string())
                .with_detail("max_delta", max_delta.to_string())
                ?;
        }
        Ok(())
    }

}