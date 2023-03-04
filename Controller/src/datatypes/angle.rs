use anyhow::{ensure, Error};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt};

pub type AngleNormalized = AngleNormalizedBase<0, 4>; // 0 - 360

pub type AngleNormalizedZeroCentered = AngleNormalizedBase<-2, 2>; // -180 - 180

pub type AngleNormalizedHalf = AngleNormalizedBase<0, 2>; // 0 - 180

pub type AngleNormalizedHalfZeroCentered = AngleNormalizedBase<-1, 1>; // -90 - 90

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
#[serde(try_from = "AngleNormalizedBaseSerde")]
#[serde(into = "AngleNormalizedBaseSerde")]
pub struct AngleNormalizedBase<const MIN_PI_DIV2: i8, const MAX_PI_DIV2: i8> {
    radians: f64,
}
impl<const MIN_PI_DIV2: i8, const MAX_PI_DIV2: i8> AngleNormalizedBase<MIN_PI_DIV2, MAX_PI_DIV2> {
    pub const fn min() -> f64 {
        (MIN_PI_DIV2 as f64) / 2.0 * std::f64::consts::PI
    }
    pub const fn max() -> f64 {
        (MAX_PI_DIV2 as f64) / 2.0 * std::f64::consts::PI
    }

    pub fn from_radians(radians: f64) -> Result<Self, Error> {
        ensure!(radians.is_finite(), "value must be finite");
        ensure!(
            (Self::min()..=Self::max()).contains(&radians),
            "value must be in range [{:.2} * pi, {:.2} * pi]",
            (MIN_PI_DIV2 as f64 / 2.0),
            (MAX_PI_DIV2 as f64 / 2.0),
        );
        Ok(Self { radians })
    }
    pub fn to_radians(&self) -> f64 {
        self.radians
    }

    pub fn from_degrees(degrees: f64) -> Result<Self, Error> {
        let min_degrees = Self::min().to_degrees();
        let max_degrees = Self::max().to_degrees();

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
impl<const MIN_PI_DIV2: i8, const MAX_PI_DIV2: i8> Eq
    for AngleNormalizedBase<MIN_PI_DIV2, MAX_PI_DIV2>
{
}
#[allow(clippy::derive_ord_xor_partial_ord)]
impl<const MIN_PI_DIV2: i8, const MAX_PI_DIV2: i8> Ord
    for AngleNormalizedBase<MIN_PI_DIV2, MAX_PI_DIV2>
{
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
impl<const MIN_PI_DIV2: i8, const MAX_PI_DIV2: i8> TryFrom<AngleNormalizedBaseSerde>
    for AngleNormalizedBase<MIN_PI_DIV2, MAX_PI_DIV2>
{
    type Error = Error;

    fn try_from(value: AngleNormalizedBaseSerde) -> Result<Self, Self::Error> {
        Self::from_radians(value.0)
    }
}
impl<const MIN_PI_DIV2: i8, const MAX_PI_DIV2: i8> Into<AngleNormalizedBaseSerde>
    for AngleNormalizedBase<MIN_PI_DIV2, MAX_PI_DIV2>
{
    fn into(self) -> AngleNormalizedBaseSerde {
        AngleNormalizedBaseSerde(self.to_radians())
    }
}
impl<const MIN_PI_DIV2: i8, const MAX_PI_DIV2: i8> fmt::Display
    for AngleNormalizedBase<MIN_PI_DIV2, MAX_PI_DIV2>
{
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{:.5}°", self.radians.to_degrees())
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct AngleNormalizedBaseSerde(f64);
