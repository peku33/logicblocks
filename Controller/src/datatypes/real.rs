use anyhow::{ensure, Error};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
#[serde(try_from = "RealSerde")]
#[serde(into = "RealSerde")]
pub struct Real(f64);
impl Real {
    pub fn from_f64(value: f64) -> Result<Self, Error> {
        ensure!(value.is_finite(), "value must be finite");
        Ok(Self(value))
    }
    pub fn to_f64(&self) -> f64 {
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
impl TryFrom<RealSerde> for Real {
    type Error = Error;

    fn try_from(value: RealSerde) -> Result<Self, Self::Error> {
        Self::from_f64(value.0)
    }
}
impl Into<RealSerde> for Real {
    fn into(self) -> RealSerde {
        RealSerde(self.to_f64())
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct RealSerde(f64);
