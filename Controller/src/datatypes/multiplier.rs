use anyhow::{ensure, Error};
use derive_more::{Add, AddAssign, Sub, SubAssign, Sum};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

// FIXME: Sub must ensure the value does not go sub-0
#[derive(
    Clone,
    Copy,
    PartialEq,
    PartialOrd,
    Add,
    Sub,
    AddAssign,
    SubAssign,
    Sum,
    Debug,
    Serialize,
    Deserialize,
)]
#[serde(try_from = "MultiplierSerde")]
#[serde(into = "MultiplierSerde")]
pub struct Multiplier(f64);
impl Multiplier {
    pub const fn zero() -> Self {
        Self(0.0)
    }
    pub const fn one() -> Self {
        Self(1.0)
    }

    pub fn from_f64(value: f64) -> Result<Self, Error> {
        ensure!(value.is_finite(), "value must be finite");
        ensure!(value >= 0.0, "value must be greater then 0.0");
        Ok(Self(value))
    }
    pub fn to_f64(&self) -> f64 {
        self.0
    }
}
impl Eq for Multiplier {}
#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for Multiplier {
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
impl TryFrom<MultiplierSerde> for Multiplier {
    type Error = Error;

    fn try_from(value: MultiplierSerde) -> Result<Self, Self::Error> {
        Self::from_f64(value.0)
    }
}
impl Into<MultiplierSerde> for Multiplier {
    fn into(self) -> MultiplierSerde {
        MultiplierSerde(self.to_f64())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct MultiplierSerde(f64);
