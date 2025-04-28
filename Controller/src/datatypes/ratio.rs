use super::real::Real;
use anyhow::{Error, ensure};
use rand::{
    Rng,
    distr::{Distribution, StandardUniform},
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
#[serde(try_from = "RatioSerde")]
#[serde(into = "RatioSerde")]
pub struct Ratio(f64);
impl Ratio {
    pub const fn zero() -> Self {
        Self(0.0)
    }
    pub const fn epsilon() -> Self {
        Self(f64::EPSILON)
    }
    pub const fn full() -> Self {
        Self(1.0)
    }

    pub fn from_f64(value: f64) -> Result<Self, Error> {
        ensure!(value.is_finite(), "value must be finite");
        ensure!(
            (0.0..=1.0).contains(&value),
            "value must be between 0.0 and 1.0"
        );
        Ok(Self(value))
    }
    pub fn to_f64(&self) -> f64 {
        self.0
    }
}
impl Eq for Ratio {}
#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for Ratio {
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        self.partial_cmp(other).unwrap()
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
