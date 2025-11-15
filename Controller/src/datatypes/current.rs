use super::real::Real;
use anyhow::Error;
use serde::{Deserialize, Serialize};
use std::fmt;
use typed_floats::NonNaNFinite;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Current {
    amperes: NonNaNFinite<f64>,
}
impl Current {
    pub const fn zero() -> Self {
        // SAFE: 0.0 is positive
        let amperes = unsafe { NonNaNFinite::<f64>::new_unchecked(0f64) };

        Self { amperes }
    }

    pub fn from_amperes(amperes: f64) -> Result<Self, Error> {
        let amperes = NonNaNFinite::<f64>::new(amperes)?;

        Ok(Self { amperes })
    }
    pub fn to_amperes(&self) -> f64 {
        self.amperes.get()
    }
}
impl fmt::Display for Current {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{:.3}A", self.to_amperes())
    }
}

impl TryFrom<Real> for Current {
    type Error = Error;

    fn try_from(value: Real) -> Result<Self, Self::Error> {
        Self::from_amperes(value.to_f64())
    }
}
impl From<Current> for Real {
    fn from(value: Current) -> Self {
        Self::from_f64(value.to_amperes()).unwrap()
    }
}
