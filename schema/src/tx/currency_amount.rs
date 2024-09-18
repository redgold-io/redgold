use std::iter::Sum;
use std::ops::{Add, Div, Mul, Sub};
use num_bigint::BigInt;
use num_traits::{FromPrimitive, ToPrimitive};
use std::str::FromStr;
use crate::constants::{DECIMAL_MULTIPLIER, MAX_COIN_SUPPLY};
use crate::{ErrorInfoContext, RgResult};
use crate::fee_validator::MIN_RDG_SATS_FEE;
use crate::structs::{CurrencyAmount, ErrorInfo, SupportedCurrency};

impl CurrencyAmount {

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

    pub fn from_fractional_cur(a: impl Into<f64>, cur: SupportedCurrency) -> RgResult<Self> {
        let into = a.into();
        let mut res = match cur {
            SupportedCurrency::Redgold => {Self::from_fractional(into)}
            SupportedCurrency::Bitcoin => {Self::from_fractional(into)}
            SupportedCurrency::Ethereum => {Ok(Self::from_eth_fractional(into))}
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

    pub fn min_fee() -> Self {
        Self::from(MIN_RDG_SATS_FEE)
    }

    pub fn bigint_amount(&self) -> Option<BigInt>  {
        self.string_amount.as_ref().map(|s| BigInt::from_str(s).ok()).flatten()
    }
    pub fn to_fractional(&self) -> f64 {
        let curr = self.currency_or();
        if curr == SupportedCurrency::Ethereum {
            self.bigint_fractional().unwrap_or(0f64)
        } else {
            self.to_fractional_std()
        }
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