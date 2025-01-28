use crate::helpers::with_metadata_hashable::WithMetadataHashable;
use crate::structs::{Address, SupportedCurrency, Transaction};
use itertools::Itertools;

pub const MIN_RDG_SATS_FEE: i64 = 1000;

pub trait TransactionFeeValidator {
    fn validate_fee(&self, addresses: &Vec<Address>) -> bool;
    fn validate_fee_only(&self, addresses: &Vec<Address>) -> bool;
}

pub trait ResolvedTransactionFeeValidator {
    fn validate_resolved_fee(&self, addresses: &Vec<Address>, max_parent_time: i64) -> bool;
}

impl ResolvedTransactionFeeValidator for Transaction {
    fn validate_resolved_fee(&self, addresses: &Vec<Address>, max_parent_time: i64) -> bool {
        let small_num_outputs = self.outputs.len() < 5;
        let total_amount = self.output_amount_total().to_fractional();
        let min_stake = total_amount >= 1.0;
        let matches_zero_fee_condition = min_stake && small_num_outputs;
        if self.validate_fee_only(addresses) {
            return true;
        }

        if matches_zero_fee_condition {
            let self_time = self.time().cloned().unwrap_or(0);
            let delta = self_time - max_parent_time;
            let mut min_expected_delta = 30 * 1000;
            if !min_stake {
                min_expected_delta = ((min_expected_delta as f64) / total_amount) as i64;
            }

            if delta > min_expected_delta {
                return true;
            }
        }
        false
    }
}


impl TransactionFeeValidator for Transaction {
    fn validate_fee(&self, addresses: &Vec<Address>) -> bool {

        let small_num_outputs = self.outputs.len() < 5;
        let matches_zero_fee_condition = self.output_amount_total().to_fractional() >= 1.0 && small_num_outputs;
        matches_zero_fee_condition || self.validate_fee_only(addresses)
    }

    fn validate_fee_only(&self, addresses: &Vec<Address>) -> bool {
        let value = self.output_address_amounts_opt()
            .filter(|(address, amount)| {
                addresses.contains(address) && amount.currency_or() == SupportedCurrency::Redgold
            }).map(|(_, amount)| amount.amount).sum::<i64>();
        let fee_condition = value >= MIN_RDG_SATS_FEE;
        fee_condition
    }
}