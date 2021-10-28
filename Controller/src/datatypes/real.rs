use anyhow::{ensure, Error};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
#[serde(try_from = "f64")]
#[serde(into = "f64")]
pub struct Real(f64);
impl Real {
    pub fn from_f64(inner: f64) -> Self {
        Self::try_from(inner).unwrap()
    }
    pub fn as_f64(&self) -> f64 {
        self.0
    }
}
impl Eq for Real {}
#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for Real {
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
impl TryFrom<f64> for Real {
    type Error = Error;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        ensure!(value.is_finite(), "value must be finite");
        Ok(Self(value))
    }
}
impl Into<f64> for Real {
    fn into(self) -> f64 {
        self.0
    }
}
