use anyhow::{ensure, Error};
use derive_more::{Add, AddAssign, Sub, SubAssign, Sum};
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};

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
        ensure!(value >= 0.0, "value must be greater then 0.0");
        Ok(Self(value))
    }
}
impl Into<f64> for Multiplier {
    fn into(self) -> f64 {
        self.0
    }
}
