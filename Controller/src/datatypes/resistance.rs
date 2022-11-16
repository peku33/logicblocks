use anyhow::{ensure, Error};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
#[serde(into = "f64")]
pub struct Resistance {
    ohms: f64,
}
impl Resistance {
    pub const fn zero() -> Self {
        Self { ohms: 0.0 }
    }
    pub const fn infinity() -> Self {
        Self {
            ohms: f64::INFINITY,
        }
    }

    pub fn from_ohms(ohms: f64) -> Result<Self, Error> {
        ensure!(
            ohms.is_finite() || ohms == f64::INFINITY,
            "ohms must be [0.0 to +InF]"
        );
        Ok(Self { ohms })
    }
    pub fn to_ohms(&self) -> f64 {
        self.ohms
    }
}
impl Eq for Resistance {}
#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for Resistance {
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
impl Into<f64> for Resistance {
    fn into(self) -> f64 {
        self.to_ohms()
    }
}
