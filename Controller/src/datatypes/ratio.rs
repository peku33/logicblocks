use anyhow::{ensure, Error};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
#[serde(try_from = "f64")]
#[serde(into = "f64")]
pub struct Ratio(f64);
impl Ratio {
    pub const fn zero() -> Self {
        Self(0.0)
    }
    pub const fn epsilon() -> Self {
        Self(f64::EPSILON)
    }
    pub const fn full() -> Self {
        Self(1.0)
    }

    pub fn from_f64(value: f64) -> Result<Self, Error> {
        ensure!(value.is_finite(), "value must be finite");
        ensure!(
            (0.0..=1.0).contains(&value),
            "value must be between 0.0 and 1.0"
        );
        Ok(Self(value))
    }
    pub fn as_f64(&self) -> f64 {
        self.0
    }
}
impl Eq for Ratio {}
#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for Ratio {
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
impl TryFrom<f64> for Ratio {
    type Error = Error;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::from_f64(value)
    }
}
impl Into<f64> for Ratio {
    fn into(self) -> f64 {
        self.as_f64()
    }
}
