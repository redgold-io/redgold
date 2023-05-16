use crate::structs::TrustData;

/// Level of precision here is 3 digits. I.e. review rating of 90.5 is 905.
const DEFAULT_TRUST_DIVISOR: f64 = 1000.0;

pub trait FloatRoundedConvert {
    fn to_rounded_i64(&self) -> i64;
}

pub trait FloatRoundedConverti64 {
    fn to_rounded_f64(&self) -> f64;
}

impl FloatRoundedConvert for f64 {
    fn to_rounded_i64(&self) -> i64 {
        (*self * DEFAULT_TRUST_DIVISOR) as i64
    }
}

impl FloatRoundedConverti64 for i64 {
    fn to_rounded_f64(&self) -> f64 {
        (*self as f64) / DEFAULT_TRUST_DIVISOR
    }
}

impl TrustData {
    pub fn with_label(&mut self, label: f64) -> &mut Self {
        self.label = label.to_rounded_i64();
        self
    }
    pub fn label(&self) -> f64 {
        self.label.to_rounded_f64()
    }
}