use std::str::FromStr;
use ethers::prelude::Transaction;
use ethers::prelude::transaction::eip2718::TypedTransaction;
use num_bigint::BigInt;
use num_traits::Signed;
use redgold_schema::{error_info, ErrorInfoContext, RgResult, SafeOption, structs};
use redgold_schema::helpers::easy_json::{EasyJson, EasyJsonDeser};
use redgold_schema::observability::errors::EnhanceErrorInfo;
use redgold_schema::structs::{CurrencyAmount, NetworkEnvironment};
use crate::eth::eth_wallet::EthWalletWrapper;
use crate::eth::historical_client::EthHistoricalClient;

impl EthWalletWrapper {

    pub fn translate_serialized_tx(payload: String) -> RgResult<TypedTransaction> {
        let tx = payload.json_from::<TypedTransaction>()?;
        Ok(tx)
    }

    pub fn decode_rlp_tx(payload: Vec<u8>) -> RgResult<Transaction> {
        let res = ethers::utils::rlp::decode::<Transaction>(&payload).error_info("rlp decode failure")?;
        Ok(res)
    }

    pub async fn validate_eth_fulfillment(
        fulfills: Vec<(structs::Address, CurrencyAmount)>,
        typed_tx_payload: &String,
        signing_data: &Vec<u8>,
        network: &NetworkEnvironment,
        w: &EthWalletWrapper
    ) -> RgResult<()> {
        let mut tx = typed_tx_payload.json_from::<TypedTransaction>()?;
        tx.set_chain_id(EthHistoricalClient::chain_id(&network).id());
        let to = tx.to_addr().ok_msg("to address missing")?;
        let amount = tx.value().ok_msg("value missing")?;
        let amount_str = amount.to_string();
        let amount_bigint = BigInt::from_str(&amount_str).error_info("bigint from str amount parse failure")?;
        let has_match = fulfills.iter()
            .map(|(f_addr, f_amt)|
                f_addr.render_string()
                    .and_then(|a| Self::parse_address(&a))
                    .map(|a| &a == to)
                    .and_then(|b| f_amt.bigint_amount().clone().ok_msg("Missing bigint amount").map(|a| {
                        let delta = a.clone() - amount_bigint.clone();
                        let delta = delta.abs();
                        let within_reasonable_range = delta < BigInt::from(1_000_000_000_000_000u64); // 1e15 as an integer
                        within_reasonable_range
                    } && b))
            ).collect::<RgResult<Vec<bool>>>()?.iter().any(|b| *b);
        if !has_match {
            return Err(error_info("fulfillment does not match transaction"))
                .with_detail("fulfills", fulfills.json_or())
                .with_detail("to_address", to.to_string())
                .with_detail("amount_str", amount_str);
        }
        let signing = Self::signing_data(&tx)?;
        if signing != *signing_data && network.is_main_stage_network() {
            return Err(error_info("signing data does not match transaction"));
        }

        let fee_est = w.get_gas_cost_estimate(&tx).await?;
        let act_gas = tx.gas().ok_msg("gas missing")?;
        let act_gas_cost = tx.gas_price().ok_msg("gas price missing")?;
        let act_fee = act_gas * act_gas_cost;
        let act_fee_ca = CurrencyAmount::from_eth_bigint_string(act_fee.to_string());
        if act_fee_ca > fee_est.clone()*10 {
            return Err(error_info("fee estimate does not match transaction"))
                .with_detail("fee_estimate", fee_est.json_or())
                .with_detail("act_fee", act_fee_ca.json_or());
        }

        Ok(())
    }

}