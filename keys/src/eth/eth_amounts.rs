use ethers::middleware::Middleware;
use ethers::prelude::transaction::eip2718::TypedTransaction;
use redgold_schema::{ErrorInfoContext, RgResult};
use redgold_schema::structs::{CurrencyAmount, NetworkEnvironment};
use crate::eth::eth_wallet::EthWalletWrapper;

impl EthWalletWrapper {

    pub async fn get_gas_cost_estimate(&self, tx: &TypedTransaction) -> RgResult<CurrencyAmount> {
        let gas = self.provider.estimate_gas(tx, None).await.error_info("gas estimate failure")?;
        Ok(CurrencyAmount::from_eth_bigint_string(gas.to_string()))
    }

    pub async fn get_gas_price(&self) -> RgResult<CurrencyAmount> {
        let gas = self.provider.get_gas_price().await.error_info("gas estimate failure")?;
        Ok(CurrencyAmount::from_eth_bigint_string(gas.to_string()))
    }

    pub async fn get_fee_estimate(&self) -> RgResult<CurrencyAmount> {
        Ok(self.get_gas_price().await? * Self::gas_cost_fixed_normal())
    }

    pub fn test_send_amount_typed() -> CurrencyAmount {
        // 0.000108594791676 originally as a fee from a testnet transaction (earlier)
        // 0.00128623 originally as a fee from a testnet transaction (current)
        let fee = 0.0005;
        CurrencyAmount::from_eth_fractional(fee)
    }

    pub fn stake_test_amount_typed() -> CurrencyAmount {
        // 0.000108594791676 originally as a fee from a testnet transaction
        let fee = 0.0300;
        CurrencyAmount::from_eth_fractional(fee)
    }

    // TODO: Set by environment.
    pub fn gas_price_fixed_normal_testnet() -> CurrencyAmount {
        // Fee: 0.000171425329026 for 21k gas used * below value
        // 8163110906 for ^
        // Higher seen:
        // 13531134318
        // 23531134318
        // 43531134318
        // 112793670539 -> 0.00236
        // 212793670539 -> 0.0046
        // 412793670539 -> 0.008
        CurrencyAmount::from_eth_bigint_string("412793670539")
    }

    pub fn gas_price_fixed_normal_mainnet() -> CurrencyAmount {
        CurrencyAmount::from_eth_bigint_string("4127936705")
    }

    pub fn gas_price_fixed_normal_by_env(env: &NetworkEnvironment) -> CurrencyAmount {
        if env.is_main() {
            Self::gas_price_fixed_normal_mainnet()
        } else {
            Self::gas_price_fixed_normal_testnet()
        }
    }

    pub fn gas_cost_fixed_normal() -> CurrencyAmount {
        // Fee: 0.000171425329026 for 21k gas used * below value
        CurrencyAmount::from_eth_bigint_string("21000")
    }

    pub fn fee_fixed_normal_testnet() -> CurrencyAmount {
        // Fee: 0.000171425329026 for 21k gas used * below value
        Self::gas_cost_fixed_normal() * Self::gas_price_fixed_normal_testnet()
    }

    pub fn fee_fixed_normal_mainnet() -> CurrencyAmount {
        // Fee: 0.000171425329026 for 21k gas used * below value
        Self::gas_cost_fixed_normal() * Self::gas_price_fixed_normal_mainnet()
    }

    pub fn fee_fixed_normal_by_env(env: &NetworkEnvironment) -> CurrencyAmount {
        if env.is_main() {
            Self::fee_fixed_normal_mainnet()
        } else {
            Self::fee_fixed_normal_testnet()
        }
    }



}


#[test]
pub fn test_eth_wallet() {
    let f = EthWalletWrapper::fee_fixed_normal_testnet().to_fractional();
    println!("fee_fixed_normal: {}", f);
    println!("fee_fixed_normalusd: {}", f * 2600.0);
     let f = EthWalletWrapper::fee_fixed_normal_mainnet().to_fractional();
    println!("fee_fixed_normal2: {}", f);
    println!("fee_fixed_normalusd2: {}", f * 2600.0);

}