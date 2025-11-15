use super::real::Real;
use anyhow::Error;
use serde::{Deserialize, Serialize};
use std::fmt;
use typed_floats::Positive;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Resistance {
    ohms: Positive<f64>,
}
impl Resistance {
    pub const fn zero() -> Self {
        // SAFE: 0.0 is positive
        let ohms = unsafe { Positive::<f64>::new_unchecked(0f64) };

        Self { ohms }
    }
    pub const fn infinity() -> Self {
        // SAFE: Infinity is positive
        let ohms = unsafe { Positive::<f64>::new_unchecked(f64::INFINITY) };

        Self { ohms }
    }

    pub fn from_ohms(ohms: f64) -> Result<Self, Error> {
        let ohms = Positive::<f64>::new(ohms)?;

        Ok(Self { ohms })
    }
    pub fn to_ohms(&self) -> f64 {
        self.ohms.get()
    }
}
impl fmt::Display for Resistance {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{:.3}Ω", self.to_ohms())
    }
}

impl TryFrom<Real> for Resistance {
    type Error = Error;

    fn try_from(value: Real) -> Result<Self, Self::Error> {
        Self::from_ohms(value.to_f64())
    }
}
impl TryFrom<Resistance> for Real {
    type Error = Error;

    fn try_from(value: Resistance) -> Result<Self, Self::Error> {
        Self::from_f64(value.to_ohms())
    }
}
