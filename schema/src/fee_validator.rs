use std::collections::HashSet;
use itertools::Itertools;
use crate::structs::{Address, Seed, SupportedCurrency, Transaction};

pub const MIN_RDG_SATS_FEE: i64 = 1000;

pub trait TransactionFeeValidator {
    fn validate_fee(&self, addresses: &Vec<Address>) -> bool;
}

impl TransactionFeeValidator for Transaction {
    fn validate_fee(&self, addresses: &Vec<Address>) -> bool {
        let value = self.output_address_amounts_opt()
            .filter(|(address, amount)| {
                addresses.contains(address) && amount.currency_or() == SupportedCurrency::Redgold
        }).map(|(address, amount)| amount.amount).sum::<i64>();
        value >= MIN_RDG_SATS_FEE
    }
}