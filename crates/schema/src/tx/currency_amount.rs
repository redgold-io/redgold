use crate::constants::{DECIMAL_MULTIPLIER, MAX_COIN_SUPPLY, NANO_DECIMAL_MULTIPLIER, PICO_DECIMAL_MULTIPLIER};
use crate::fee_validator::MIN_RDG_SATS_FEE;
use crate::structs::{CurrencyAmount, CurrencyId, ErrorInfo, NetworkEnvironment, SupportedCurrency};
use crate::{ErrorInfoContext, RgResult};
use num_bigint::BigInt;
use num_traits::{FromPrimitive, ToPrimitive};
use std::iter::Sum;
use std::ops::{Add, Div, Mul, Sub, SubAssign};
use std::str::FromStr;


pub trait RenderCurrencyAmountDecimals {
    fn render_currency_amount_8_decimals(&self) -> String;
    fn render_currency_amount_2_decimals(&self) -> String;
}

impl RenderCurrencyAmountDecimals for f64 {
    fn render_currency_amount_8_decimals(&self) -> String {
        format!("{:.8}", self)
    }

    fn render_currency_amount_2_decimals(&self) -> String {
        format!("{:.2}", self)
    }
}


impl CurrencyAmount {

    pub fn render_8_decimals(&self) -> String {
        self.to_fractional().render_currency_amount_8_decimals()
    }

    pub fn amount_i64(&self) -> i64 {
        self.amount
    }
    pub fn from_fractional(a: impl Into<f64>) -> Result<Self, ErrorInfo> {
        let a = a.into();
        if a <= 0f64 {
            Err(ErrorInfo::error_info("Invalid negative or zero transaction amount"))?
        }
        if a > MAX_COIN_SUPPLY as f64 {
            Err(ErrorInfo::error_info("Invalid transaction amount"))?
        }
        let amount = (a * (DECIMAL_MULTIPLIER as f64)) as i64;
        let mut a = CurrencyAmount::default();
        a.amount = amount;
        Ok(a)
    }
    pub fn from_fractional_basis(a: impl Into<f64>, basis: i64) -> Result<Self, ErrorInfo> {
        let a = a.into();
        if a <= 0f64 {
            Err(ErrorInfo::error_info("Invalid negative or zero transaction amount"))?
        }
        let amount = (a * (basis as f64)) as i64;
        let mut a = CurrencyAmount::default();
        a.amount = amount;
        Ok(a)
    }

    pub fn from_fractional_cur(a: impl Into<f64>, cur: SupportedCurrency) -> RgResult<Self> {
        let into = a.into();
        let mut res = match cur {
            SupportedCurrency::Redgold => {Self::from_fractional(into)}
            SupportedCurrency::Bitcoin => {Self::from_fractional(into)}
            SupportedCurrency::Ethereum => {Ok(Self::from_eth_fractional(into))}
            SupportedCurrency::Solana => {Self::from_fractional_basis(into, NANO_DECIMAL_MULTIPLIER)}
            SupportedCurrency::Monero => {Self::from_fractional_basis(into, PICO_DECIMAL_MULTIPLIER)}
            _ => Err(ErrorInfo::error_info("Invalid currency"))
        }?;
        res.currency = Some(cur as i32);
        Ok(res)
    }

    pub fn from_usd(a: impl Into<f64>) -> RgResult<Self> {
        let a = a.into();
        if a <= 0f64 {
            Err(ErrorInfo::error_info("Invalid negative or zero transaction amount"))?
        }
        let amount = (a * (DECIMAL_MULTIPLIER as f64)) as i64;
        let mut a = CurrencyAmount::default();
        a.amount = amount;
        a.currency = Some(SupportedCurrency::Usd as i32);
        Ok(a)
    }


    // Workaround for dealing with u64's etc, drop from e18 precision to e8 precision
    pub fn bigint_offset_denomination() -> BigInt {
        BigInt::from(10_u64.pow(10))
    }
    pub fn bigint_actual_denomination() -> BigInt {
        BigInt::from(10_u64.pow(18))
    }
    pub fn bigint_actual_float_denomination() -> f64 {
        1e18
    }

    pub fn amount_i64_or(&self) -> i64 {
        let curr = self.currency_or();
        if curr == SupportedCurrency::Ethereum {
            self.bigint_amount_offset().and_then(|b| b.to_i64()).unwrap_or(0)
        } else {
            self.amount
        }
    }

    fn bigint_amount_offset(&self) -> Option<BigInt> {
        self.bigint_amount()
            .map(|a| a / Self::bigint_offset_denomination())
    }

    fn bigint_fractional(&self) -> Option<f64> {
        self.bigint_amount()
            .and_then(|a| a.to_f64())
            .map(|a| a / Self::bigint_actual_float_denomination())
    }

    pub fn currency_or(&self) -> SupportedCurrency {
        self.currency
            .and_then(|c| SupportedCurrency::from_i32(c))
            .unwrap_or(SupportedCurrency::Redgold)
    }

    pub fn is_rdg(&self) -> bool {
        self.currency_or() == SupportedCurrency::Redgold
    }

    pub fn is_zero(&self) -> bool {
        self.to_fractional() == 0f64
    }

    pub fn min_fee() -> Self {
        Self::from(MIN_RDG_SATS_FEE)
    }

    pub fn std_pool_fee() -> Self {
        Self::from(100_000)
    }

    pub fn bigint_amount(&self) -> Option<BigInt>  {
        self.string_amount.as_ref().map(|s| BigInt::from_str(s).ok()).flatten()
    }
    pub fn to_fractional(&self) -> f64 {
        let curr = self.currency_or();
        if curr == SupportedCurrency::Ethereum {
            if let Some(decimals) = self.decimals.as_ref() {
                if let Some(b) = self.bigint_amount() {
                    if let Some(d) = BigInt::from_str(decimals).ok() {
                        return (b / d).to_f64().unwrap_or(0f64);
                    }
                }
            }
            self.bigint_fractional().unwrap_or(0f64)
        } else if curr == SupportedCurrency::Monero {
            (self.amount as f64) / (PICO_DECIMAL_MULTIPLIER as f64)
        } else if curr == SupportedCurrency::Solana {
            (self.amount as f64) / (NANO_DECIMAL_MULTIPLIER as f64)
        } else {
            self.to_fractional_std()
        }
    }

    pub fn to_1e8(&self) -> i64 {
        ((self.to_fractional() * 1e8) / (1e8f64)) as i64
    }

    fn to_fractional_std(&self) -> f64 {
        (self.amount as f64) / (DECIMAL_MULTIPLIER as f64)
    }

    pub fn to_rounded_int(&self) -> i64 {
        self.to_fractional() as i64
    }
    pub fn from(amount: i64) -> Self {
        let mut a = Self::default();
        a.amount = amount;
        a
    }
    pub fn from_currency(amount: i64, supported_currency: SupportedCurrency) -> Self {
        let mut a = Self::default();
        a.amount = amount;
        a.currency = Some(supported_currency as i32);
        a
    }
    pub fn zero(supported_currency: SupportedCurrency) -> Self {
        let mut a = Self::default();
        a.currency = Some(supported_currency as i32);
        if supported_currency == SupportedCurrency::Ethereum {
            a.string_amount = Some("0".to_string());
        }
        a
    }

    pub fn from_string(amount: String) -> Self {
        let mut a = Self::default();
        a.string_amount = Some(amount);
        a
    }


    pub fn from_btc(amount: i64) -> Self {
        let mut a = Self::from(amount);
        a.currency = Some(SupportedCurrency::Bitcoin as i32);
        a
    }
    pub fn from_eth_bigint_string(amount: impl Into<String>) -> Self {
        let mut a = Self::from_string(amount.into());
        a.currency = Some(SupportedCurrency::Ethereum as i32);
        a
    }
    pub fn from_eth_network_bigint_string_currency_id_decimals(
        amount: impl Into<String>,
        currency_id: impl Into<CurrencyId>,
        decimals: Option<String>
    ) -> Self {
        let mut a = Self::from_string(amount.into());
        a.currency = Some(SupportedCurrency::Ethereum as i32);
        a.decimals = decimals;
        a.currency_id = Some(currency_id.into());
        a
    }

    pub fn from_eth_bigint(amount: BigInt) -> Self {
        let mut a = Self::from_string(amount.to_string());
        a.currency = Some(SupportedCurrency::Ethereum as i32);
        a
    }
    pub fn from_eth_i64(amount: i64) -> Self {
        let bi = BigInt::from_i64(amount).expect("from_i64") * Self::bigint_offset_denomination();
        Self::from_eth_bigint(bi)
    }
    pub fn from_eth_fractional(amount: f64) -> Self {
        let bi = BigInt::from_f64(amount * 1e18).expect("from_f64");
        Self::from_eth_bigint(bi)
    }

    pub fn from_rdg(amount: i64) -> Self {
        let mut a = Self::from(amount);
        a.currency = Some(SupportedCurrency::Redgold as i32);
        a
    }

    pub fn from_float_string(str: &String) -> Result<Self, ErrorInfo> {
        let amount = str.parse::<f64>()
            .error_info("Invalid transaction amount")?;
        Self::from_fractional(amount)
    }
}

use std::ops::AddAssign;

impl AddAssign for CurrencyAmount {
    fn add_assign(&mut self, other: Self) {
        *self = self.clone() + other;
    }
}

impl SubAssign for CurrencyAmount {
    fn sub_assign(&mut self, other: Self) {
        *self = self.clone() - other;
    }
}

impl Add for CurrencyAmount {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut a = self.clone();
        assert_eq!(a.currency_or(), rhs.currency_or());
        if let Some(ba) = self.bigint_amount() {
            let rhs_ba = rhs.bigint_amount().expect("rhs_bigint_amount");
            let added = ba + rhs_ba;
            a.string_amount = Some(added.to_string());
        } else {
            a.amount += rhs.amount;
        }
        a
    }
}

impl Sub for CurrencyAmount {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut a = self.clone();
        assert_eq!(a.currency_or(), rhs.currency_or());
        if let Some(ba) = self.bigint_amount() {
            let rhs_ba = rhs.bigint_amount().expect("rhs_bigint_amount");
            let added = ba - rhs_ba;
            a.string_amount = Some(added.to_string());
        } else {
            a.amount -= rhs.amount;
        }
        a
    }
}

impl Mul for CurrencyAmount {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut a = self.clone();
        assert_eq!(a.currency_or(), rhs.currency_or());
        if let (Some(ba), Some(rhs_ba)) = (self.bigint_amount(), rhs.bigint_amount()) {
            let added = ba * rhs_ba;
            a.string_amount = Some(added.to_string());
        } else {
            a.amount *= rhs.amount;
        }
        a
    }

}
impl Mul<i64> for CurrencyAmount {
    type Output = Self;

    fn mul(self, rhs: i64) -> Self::Output {
        let mut a = self.clone();
        if let Some(ba) = self.bigint_amount() {
            let added = ba * rhs;
            a.string_amount = Some(added.to_string());
        } else {
            a.amount *= rhs;
        }
        a
    }

}

impl Div for CurrencyAmount {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        let mut a = self.clone();
        assert_eq!(a.currency_or(), rhs.currency_or());
        if let Some(ba) = self.bigint_amount() {
            let rhs_ba = rhs.bigint_amount().expect("rhs_bigint_amount");
            let added = ba / rhs_ba;
            a.string_amount = Some(added.to_string());
        } else {
            a.amount /= rhs.amount;
        }
        a
    }

}


impl Sum for CurrencyAmount {
    fn sum<I: Iterator<Item = CurrencyAmount>>(iter: I) -> Self {
        iter.reduce(|a, b| a + b).unwrap_or(CurrencyAmount::default())
    }
}


use std::cmp::Ordering;

impl PartialOrd for CurrencyAmount {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if let (Some(ba), Some(ob)) = (self.bigint_amount(), other.bigint_amount()) {
            ba.partial_cmp(&ob)
        } else {
            self.amount.partial_cmp(&other.amount)
        }
    }
}

impl Ord for CurrencyAmount {
    fn cmp(&self, other: &Self) -> Ordering {
        if let (Some(ba), Some(ob)) = (self.bigint_amount(), other.bigint_amount()) {
            ba.cmp(&ob)
        } else {
            self.amount.cmp(&other.amount)
        }
    }
}


impl CurrencyAmount {

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
    pub fn eth_gas_price_fixed_normal_testnet() -> CurrencyAmount {
        // Fee: 0.000171425329026 for 21k gas used * below value
        // 8163110906 for ^
        // Higher seen:
        // 13531134318
        // 23531134318
        // 43531134318
        // 112793670539 -> 0.00236
        // 212793670539 -> 0.0046
        // 412793670539 -> 0.008
        // CurrencyAmount::from_eth_bigint_string("412793670539")
        CurrencyAmount::from_eth_bigint_string("12793670539")
    }

    pub fn eth_gas_price_fixed_normal_mainnet() -> CurrencyAmount {
        // CurrencyAmount::from_eth_bigint_string("4127936705"); // 0.27cents
        CurrencyAmount::from_eth_bigint_string("16511746820") // 1.08cents
    }

    pub fn gas_price_fixed_normal_by_env(env: &NetworkEnvironment) -> CurrencyAmount {
        if env.is_main() {
            Self::eth_gas_price_fixed_normal_mainnet()
        } else {
            Self::eth_gas_price_fixed_normal_testnet()
        }
    }

    pub fn eth_estimated_tx_gas_cost_fixed_normal() -> CurrencyAmount {
        // Fee: 0.000171425329026 for 21k gas used * below value
        CurrencyAmount::from_eth_bigint_string("21000")
    }

    pub fn eth_fee_fixed_normal_testnet() -> CurrencyAmount {
        // Fee: 0.000171425329026 for 21k gas used * below value
        Self::eth_estimated_tx_gas_cost_fixed_normal() * Self::eth_gas_price_fixed_normal_testnet()
    }

    pub fn eth_fee_fixed_normal_mainnet() -> CurrencyAmount {
        // Fee: 0.000171425329026 for 21k gas used * below value
        Self::eth_estimated_tx_gas_cost_fixed_normal() * Self::eth_gas_price_fixed_normal_mainnet()
    }

    pub fn eth_fee_fixed_normal_by_env(env: &NetworkEnvironment) -> CurrencyAmount {
        if env.is_main() {
            Self::eth_fee_fixed_normal_mainnet()
        } else {
            Self::eth_fee_fixed_normal_testnet()
        }
    }

}