use anyhow::{ensure, Error};
use derive_more::{Add, AddAssign, Sub, SubAssign, Sum};
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};

#[derive(
    Serialize,
    Deserialize,
    PartialOrd,
    PartialEq,
    Add,
    Sub,
    AddAssign,
    SubAssign,
    Sum,
    Copy,
    Clone,
    Debug,
)]
#[serde(try_from = "f64")]
#[serde(into = "f64")]
pub struct Multiplier(f64);
impl Multiplier {
    pub fn new(value: f64) -> Self {
        let self_: Self = value.try_into().unwrap();
        self_
    }

    pub const fn zero() -> Self {
        Self(0.0)
    }
    pub const fn one() -> Self {
        Self(1.0)
    }
}
impl TryFrom<f64> for Multiplier {
    type Error = Error;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        ensure!(value.is_finite(), "value must be finite");
        ensure!(value >= 0.0, "value must be between 0.0 and 1.0");
        Ok(Self(value))
    }
}
impl Into<f64> for Multiplier {
    fn into(self) -> f64 {
        self.0
    }
}
