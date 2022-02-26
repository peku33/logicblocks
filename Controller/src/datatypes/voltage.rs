use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
#[serde(into = "f64")]
pub struct Voltage {
    volts: f64,
}
impl Voltage {
    pub fn from_volts(volts: f64) -> Self {
        assert!(volts.is_finite(), "volts must be finite");
        Self { volts }
    }
    pub fn to_volts(&self) -> f64 {
        self.volts
    }
}
impl Eq for Voltage {}
#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for Voltage {
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
impl Into<f64> for Voltage {
    fn into(self) -> f64 {
        self.to_volts()
    }
}
