use super::real::Real;
use anyhow::Error;
use serde::{Deserialize, Serialize};
use std::fmt;
use typed_floats::PositiveFinite;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Pressure {
    pascals: PositiveFinite<f64>,
}
impl Pressure {
    pub fn from_pascals(pascals: f64) -> Result<Self, Error> {
        let pascals = PositiveFinite::<f64>::new(pascals)?;

        Ok(Self { pascals })
    }
    pub fn to_pascals(&self) -> f64 {
        self.pascals.get()
    }

    pub fn from_bar(bar: f64) -> Result<Self, Error> {
        Self::from_pascals(bar * 1e5)
    }
    pub fn to_bar(&self) -> f64 {
        self.to_pascals() * 1e-5
    }

    pub fn from_millibars_hectopascals(millibars_hectopascals: f64) -> Result<Self, Error> {
        Self::from_pascals(millibars_hectopascals * 1e2)
    }
    pub fn to_millibars_hectopascals(&self) -> f64 {
        self.to_pascals() * 1e-2
    }
}
impl fmt::Display for Pressure {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{:.3}Pa", self.to_pascals())
    }
}

impl TryFrom<Real> for Pressure {
    type Error = Error;

    fn try_from(value: Real) -> Result<Self, Self::Error> {
        Self::from_pascals(value.to_f64())
    }
}
impl From<Pressure> for Real {
    fn from(value: Pressure) -> Self {
        Self::from_f64(value.to_pascals()).unwrap()
    }
}
