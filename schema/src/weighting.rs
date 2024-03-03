use crate::structs::Weighting;


const CONVERSION: f64 = 1e3;

impl Weighting {
    pub fn to_float(&self) -> f64 {
        (self.value as f64) / CONVERSION
    }
    pub fn from_float(f: f64) -> Weighting {
        Weighting {
            value: (f * CONVERSION) as i64
        }
    }
}