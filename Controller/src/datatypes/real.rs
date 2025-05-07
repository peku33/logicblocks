use anyhow::{Error, ensure};
use derive_more::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(
    Clone,
    Copy,
    PartialEq,
    PartialOrd,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Mul,
    MulAssign,
    Debug,
    Serialize,
    Deserialize,
)]
#[mul(forward)]
#[serde(try_from = "RealSerde")]
#[serde(into = "RealSerde")]
pub struct Real(f64);
impl Real {
    pub const fn zero() -> Self {
        Self(0.0)
    }

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

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct RealSerde(f64);
impl TryFrom<RealSerde> for Real {
    type Error = Error;

    fn try_from(value: RealSerde) -> Result<Self, Self::Error> {
        Self::from_f64(value.0)
    }
}
impl From<Real> for RealSerde {
    fn from(value: Real) -> Self {
        RealSerde(value.to_f64())
    }
}
