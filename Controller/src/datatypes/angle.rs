use anyhow::{ensure, Error};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt};

const ZERO: f64 = 0.0; // TODO: compiler bug https://github.com/rust-lang/rust/issues/98813

pub type AngleNormalized = AngleNormalizedBase<ZERO, { 2.0 * std::f64::consts::PI }>;

pub type AngleNormalizedZeroCentered =
    AngleNormalizedBase<{ -std::f64::consts::PI }, { std::f64::consts::PI }>;

pub type AngleNormalizedHalf = AngleNormalizedBase<ZERO, { std::f64::consts::PI }>;

pub type AngleNormalizedHalfZeroCentered =
    AngleNormalizedBase<{ -std::f64::consts::PI / 2.0 }, { std::f64::consts::PI / 2.0 }>;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
#[serde(try_from = "AngleNormalizedBaseSerde")]
#[serde(into = "AngleNormalizedBaseSerde")]
pub struct AngleNormalizedBase<const MIN: f64, const MAX: f64> {
    radians: f64,
}
impl<const MIN: f64, const MAX: f64> AngleNormalizedBase<MIN, MAX> {
    pub fn from_radians(radians: f64) -> Result<Self, Error> {
        ensure!(radians.is_finite(), "value must be finite");
        ensure!(
            (MIN..=MAX).contains(&radians),
            "value must be in range [{:.2} * pi, {:.2} * pi]",
            MIN / std::f64::consts::PI,
            MAX / std::f64::consts::PI,
        );
        Ok(Self { radians })
    }
    pub fn to_radians(&self) -> f64 {
        self.radians
    }

    pub fn from_degrees(degrees: f64) -> Result<Self, Error> {
        let min_degrees = MIN.to_degrees();
        let max_degrees = MAX.to_degrees();

        ensure!(
            (min_degrees..max_degrees).contains(&degrees),
            "value must be in range [{:.2}, {:.2}]",
            min_degrees,
            max_degrees,
        );
        let radians = degrees.to_radians();
        Ok(Self { radians })
    }
}
impl<const MIN: f64, const MAX: f64> Eq for AngleNormalizedBase<MIN, MAX> {}
#[allow(clippy::derive_ord_xor_partial_ord)]
impl<const MIN: f64, const MAX: f64> Ord for AngleNormalizedBase<MIN, MAX> {
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
impl<const MIN: f64, const MAX: f64> TryFrom<AngleNormalizedBaseSerde>
    for AngleNormalizedBase<MIN, MAX>
{
    type Error = Error;

    fn try_from(value: AngleNormalizedBaseSerde) -> Result<Self, Self::Error> {
        Self::from_radians(value.0)
    }
}
impl<const MIN: f64, const MAX: f64> Into<AngleNormalizedBaseSerde>
    for AngleNormalizedBase<MIN, MAX>
{
    fn into(self) -> AngleNormalizedBaseSerde {
        AngleNormalizedBaseSerde(self.to_radians())
    }
}
impl<const MIN: f64, const MAX: f64> fmt::Display for AngleNormalizedBase<MIN, MAX> {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{:.5}°", self.radians.to_degrees())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct AngleNormalizedBaseSerde(f64);
