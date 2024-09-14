use itertools::Itertools;
use redgold_schema::RgResult;
use redgold_schema::structs::{Address, CurrencyAmount, DepositRequest, Output, PortfolioFulfillmentParams, PortfolioRequest, PortfolioWeighting, StakeDeposit, StakeRequest, StandardData, StandardRequest, SupportedCurrency, Weighting};
use crate::core::transact::tx_builder_supports::TransactionBuilder;

impl TransactionBuilder {

    pub fn with_portfolio_request(
        &mut self,
        weights: Vec<(SupportedCurrency, f64)>,
        redgold_to_party_payment_amount: &CurrencyAmount,
        party_address: &Address
    ) -> Self {
        let updated = self.with_output(party_address, redgold_to_party_payment_amount);
        let mut pr = PortfolioRequest::default();
        let transformed_weights = weights.into_iter().map(|(c, w)| {
            let mut wee = PortfolioWeighting::default();
            let w2 = Weighting::from_float(w);
            wee.weight = Some(w2);
            wee.currency = Some(c as i32);
            wee
        }).collect_vec();
        let mut pi = redgold_schema::structs::PortfolioInfo::default();
        pi.portfolio_weightings = transformed_weights;
        pr.portfolio_info = Some(pi);
        let mut o = Output::default();
        o.address = updated.input_addresses.clone().get(0).cloned();
        let mut data = StandardData::default();
        let mut req = StandardRequest::default();
        req.portfolio_request = Some(pr);
        data.standard_request = Some(req);
        o.data = Some(data);
        updated.transaction.outputs.push(o);
        updated.clone()
    }

    pub fn with_portfolio_stake_fullfillment(&mut self,
                                             stake_control_address: &Address,
                                             external_address: &Address,
                                             external_amount: &CurrencyAmount,
                                             pool_address: &Address,
                                             pool_fee: &CurrencyAmount,
    ) -> &mut Self {
        self.with_output(pool_address, pool_fee);
        self.with_last_output_stake();
        let mut o = Output::default();
        o.address = Some(stake_control_address.clone());
        let mut d = StandardData::default();
        let mut lq = StakeRequest::default();
        let mut deposit = StakeDeposit::default();
        deposit.portfolio_fulfillment_params = Some(PortfolioFulfillmentParams::default());
        let mut dr = DepositRequest::default();
        let mut external = external_address.clone();
        external.mark_external();
        dr.address = Some(external);
        dr.amount = Some(external_amount.clone());
        deposit.deposit = Some(dr);
        lq.deposit = Some(deposit);

        let mut sr = StandardRequest::default();
        sr.stake_request = Some(lq);
        d.standard_request = Some(sr);
        o.data = Some(d);
        self.transaction.outputs.push(o);
        self
    }

}