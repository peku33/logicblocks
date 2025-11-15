use super::real::Real;
use anyhow::Error;
use serde::{Deserialize, Serialize};
use std::fmt;
use typed_floats::NonNaNFinite;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FlowVolumetric {
    cubic_meters_per_second: NonNaNFinite<f64>,
}
impl FlowVolumetric {
    pub fn from_cubic_meters_per_second(cubic_meters_per_second: f64) -> Result<Self, Error> {
        let cubic_meters_per_second = NonNaNFinite::<f64>::new(cubic_meters_per_second)?;

        Ok(Self {
            cubic_meters_per_second,
        })
    }
    pub fn to_cubic_meters_per_second(&self) -> f64 {
        self.cubic_meters_per_second.get()
    }

    pub fn from_liters_per_minute(liters_per_minute: f64) -> Result<Self, Error> {
        Self::from_cubic_meters_per_second(liters_per_minute / 1000.0 / 60.0)
    }
    pub fn to_liters_per_minute(&self) -> f64 {
        self.to_cubic_meters_per_second() * 1000.0 * 60.0
    }
}
impl fmt::Display for FlowVolumetric {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{:.3}l/min", self.to_liters_per_minute())
    }
}

impl TryFrom<Real> for FlowVolumetric {
    type Error = Error;

    fn try_from(value: Real) -> Result<Self, Self::Error> {
        Self::from_cubic_meters_per_second(value.to_f64())
    }
}
impl From<FlowVolumetric> for Real {
    fn from(value: FlowVolumetric) -> Self {
        Self::from_f64(value.to_cubic_meters_per_second()).unwrap()
    }
}
