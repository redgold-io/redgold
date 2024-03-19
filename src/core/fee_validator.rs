use std::collections::HashSet;
use itertools::Itertools;
use redgold_schema::structs::{Address, Seed, SupportedCurrency, Transaction};

const MIN_RDG_SATS_FEE: i64 = 1000;

trait TransactionFeeValidator {
    fn validate_fee(&self, seeds: Vec<&Address>) -> bool;
}

impl TransactionFeeValidator for Transaction {
    fn validate_fee(&self, addresses: Vec<&Address>) -> bool {
        let value = self.output_address_amounts_opt()
            .filter(|(address, amount)| {
                addresses.contains(address) && amount.currency() == SupportedCurrency::Redgold
        }).map(|(address, amount)| amount.amount).sum::<i64>();
        value > MIN_RDG_SATS_FEE
    }
}