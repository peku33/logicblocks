use anyhow::{ensure, Error};
use rand::{
    distributions::{Distribution, Standard},
    Rng,
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
impl TryFrom<RatioSerde> for Ratio {
    type Error = Error;

    fn try_from(value: RatioSerde) -> Result<Self, Self::Error> {
        Self::from_f64(value.0)
    }
}
impl Into<RatioSerde> for Ratio {
    fn into(self) -> RatioSerde {
        RatioSerde(self.to_f64())
    }
}
impl Distribution<Ratio> for Standard {
    fn sample<R: Rng + ?Sized>(
        &self,
        rng: &mut R,
    ) -> Ratio {
        Ratio::from_f64(rng.gen_range(0.0..=1.0)).unwrap()
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct RatioSerde(f64);
