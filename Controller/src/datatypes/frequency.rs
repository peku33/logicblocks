use super::real::Real;
use anyhow::Error;
use serde::{Deserialize, Serialize};
use std::fmt;
use typed_floats::PositiveFinite;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Frequency {
    hertz: PositiveFinite<f64>,
}
impl Frequency {
    pub const fn zero() -> Self {
        // SAFE: 0.0 is positive
        let hertz = unsafe { PositiveFinite::<f64>::new_unchecked(0f64) };

        Self { hertz }
    }

    pub fn from_hertz(hertz: f64) -> Result<Self, Error> {
        let hertz = PositiveFinite::<f64>::new(hertz)?;

        Ok(Self { hertz })
    }
    pub fn to_hertz(&self) -> f64 {
        self.hertz.get()
    }
}
impl fmt::Display for Frequency {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{:.2}Hz", self.to_hertz())
    }
}

impl TryFrom<Real> for Frequency {
    type Error = Error;

    fn try_from(value: Real) -> Result<Self, Self::Error> {
        Self::from_hertz(value.to_f64())
    }
}
impl From<Frequency> for Real {
    fn from(value: Frequency) -> Self {
        Self::from_f64(value.to_hertz()).unwrap()
    }
}
