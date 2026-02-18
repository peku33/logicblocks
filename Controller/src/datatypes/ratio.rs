use super::real::Real;
use anyhow::{Error, ensure};
use rand::{
    Rng, RngExt,
    distr::{Distribution, StandardUniform},
};
use serde::{Deserialize, Serialize};
use std::fmt;
use typed_floats::NonNaNFinite;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
#[serde(try_from = "RatioSerde")]
#[serde(into = "RatioSerde")]
pub struct Ratio(NonNaNFinite<f64>);
impl Ratio {
    pub const fn zero() -> Self {
        // SAFE: 0.0 is always finite
        let inner = unsafe { NonNaNFinite::<f64>::new_unchecked(0f64) };

        Self(inner)
    }
    pub const fn epsilon() -> Self {
        // SAFE: EPSILON is always finite
        let inner = unsafe { NonNaNFinite::<f64>::new_unchecked(f64::EPSILON) };

        Self(inner)
    }
    pub const fn full() -> Self {
        // SAFE: 1.0 is always finite
        let inner = unsafe { NonNaNFinite::<f64>::new_unchecked(1f64) };

        Self(inner)
    }

    pub fn from_f64(value: f64) -> Result<Self, Error> {
        let value = NonNaNFinite::<f64>::new(value)?;
        ensure!(
            (0.0..=1.0).contains(&value),
            "value must be between 0.0 and 1.0"
        );

        Ok(Self(value))
    }
    pub fn to_f64(&self) -> f64 {
        self.0.get()
    }
}
impl fmt::Display for Ratio {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{:.3}", self.to_f64())
    }
}

impl TryFrom<Real> for Ratio {
    type Error = Error;

    fn try_from(value: Real) -> Result<Self, Self::Error> {
        Self::from_f64(value.to_f64())
    }
}
impl From<Ratio> for Real {
    fn from(value: Ratio) -> Self {
        Self::from_f64(value.to_f64()).unwrap()
    }
}

impl Distribution<Ratio> for StandardUniform {
    fn sample<R: Rng + ?Sized>(
        &self,
        rng: &mut R,
    ) -> Ratio {
        Ratio::from_f64(rng.random_range(0.0..=1.0)).unwrap()
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct RatioSerde(f64);
impl TryFrom<RatioSerde> for Ratio {
    type Error = Error;

    fn try_from(value: RatioSerde) -> Result<Self, Self::Error> {
        Self::from_f64(value.0)
    }
}
impl From<Ratio> for RatioSerde {
    fn from(value: Ratio) -> Self {
        RatioSerde(value.to_f64())
    }
}
