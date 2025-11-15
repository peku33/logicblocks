use super::real::Real;
use anyhow::Error;
use serde::{Deserialize, Serialize};
use std::fmt;
use typed_floats::NonNaNFinite;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Voltage {
    volts: NonNaNFinite<f64>,
}
impl Voltage {
    pub const fn zero() -> Self {
        // SAFE: 0.0 is positive
        let volts = unsafe { NonNaNFinite::<f64>::new_unchecked(0f64) };

        Self { volts }
    }

    pub fn from_volts(volts: f64) -> Result<Self, Error> {
        let volts = NonNaNFinite::<f64>::new(volts)?;

        Ok(Self { volts })
    }
    pub fn to_volts(&self) -> f64 {
        self.volts.get()
    }
}
impl fmt::Display for Voltage {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{:.3}V", self.to_volts())
    }
}

impl TryFrom<Real> for Voltage {
    type Error = Error;

    fn try_from(value: Real) -> Result<Self, Self::Error> {
        Self::from_volts(value.to_f64())
    }
}
impl From<Voltage> for Real {
    fn from(value: Voltage) -> Self {
        Self::from_f64(value.to_volts()).unwrap()
    }
}
