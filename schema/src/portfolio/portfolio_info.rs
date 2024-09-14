use std::collections::HashMap;
use crate::structs::{PortfolioInfo, SupportedCurrency};

impl PortfolioInfo {
    pub fn fixed_currency_allocations(&self) -> HashMap<SupportedCurrency, f64> {
        let total = self.total_weight();
        self.portfolio_weightings.iter().flat_map(|pw| {
            pw.currency.and_then(|c| SupportedCurrency::from_i32(c)
                .and_then(|cc|
                    pw.weight.as_ref().map(|w| (cc, w.to_float() / total))
                )
            )
        }).collect()
    }
    pub fn total_weight(&self) -> f64 {
        self.portfolio_weightings.iter()
            .flat_map(|w| w.weight.as_ref())
            .map(|w| w.to_float())
            .sum()
    }
}