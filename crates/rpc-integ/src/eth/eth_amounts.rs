use crate::eth::eth_wallet::EthWalletWrapper;
use ethers::middleware::Middleware;
use ethers::prelude::transaction::eip2718::TypedTransaction;
use redgold_schema::structs::CurrencyAmount;
use redgold_schema::{ErrorInfoContext, RgResult};

impl EthWalletWrapper {

    pub async fn get_gas_cost_estimate(&self, tx: &TypedTransaction) -> RgResult<CurrencyAmount> {
        let gas = self.provider.estimate_gas(tx, None).await.error_info("get gas cost estimate failure")?;
        Ok(CurrencyAmount::from_eth_bigint_string(gas.to_string()))
    }

    pub async fn get_gas_price(&self) -> RgResult<CurrencyAmount> {
        let gas = self.provider.get_gas_price().await.error_info("get gas price estimate failure")?;
        Ok(CurrencyAmount::from_eth_bigint_string(gas.to_string()))
    }

    pub async fn get_fee_estimate(&self) -> RgResult<CurrencyAmount> {
        Ok(self.get_gas_price().await? * CurrencyAmount::eth_estimated_tx_gas_cost_fixed_normal())
    }


}


#[test]
pub fn test_eth_wallet() {
    let f = CurrencyAmount::eth_fee_fixed_normal_testnet().to_fractional();
    println!("fee_fixed_normal: {}", f);
    println!("fee_fixed_normalusd: {}", f * 2600.0);
     let f = CurrencyAmount::eth_fee_fixed_normal_mainnet().to_fractional();
    println!("fee_fixed_normal2: {}", f);
    println!("fee_fixed_normalusd2: {}", f * 2600.0);

}