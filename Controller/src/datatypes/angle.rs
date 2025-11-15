use anyhow::{Context, Error, ensure};
use serde::{Deserialize, Serialize};
use std::fmt;
use typed_floats::NonNaNFinite;

pub type AngleNormalized = AngleNormalizedBase<0, 4>; // 0 - 360

pub type AngleNormalizedZeroCentered = AngleNormalizedBase<-2, 2>; // -180 - 180

pub type AngleNormalizedHalf = AngleNormalizedBase<0, 2>; // 0 - 180

pub type AngleNormalizedHalfZeroCentered = AngleNormalizedBase<-1, 1>; // -90 - 90

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
#[serde(try_from = "AngleNormalizedBaseSerde")]
#[serde(into = "AngleNormalizedBaseSerde")]
pub struct AngleNormalizedBase<const MIN_PI_DIV2: i8, const MAX_PI_DIV2: i8> {
    radians: NonNaNFinite<f64>,
}
impl<const MIN_PI_DIV2: i8, const MAX_PI_DIV2: i8> AngleNormalizedBase<MIN_PI_DIV2, MAX_PI_DIV2> {
    pub const fn min() -> f64 {
        (MIN_PI_DIV2 as f64) / 2.0 * std::f64::consts::PI
    }
    pub const fn max() -> f64 {
        (MAX_PI_DIV2 as f64) / 2.0 * std::f64::consts::PI
    }

    pub fn from_radians(radians: f64) -> Result<Self, Error> {
        let radians = NonNaNFinite::<f64>::new(radians)?;
        ensure!(
            (Self::min()..=Self::max()).contains(&radians),
            "value must be in range [{:.2} * pi, {:.2} * pi]",
            (MIN_PI_DIV2 as f64 / 2.0),
            (MAX_PI_DIV2 as f64 / 2.0),
        );

        Ok(Self { radians })
    }
    pub fn to_radians(&self) -> f64 {
        self.radians.get()
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

        let self_ = Self::from_radians(radians).context("from_radians")?;

        Ok(self_)
    }
    pub fn to_degrees(&self) -> f64 {
        self.to_radians().to_degrees()
    }
}
impl<const MIN_PI_DIV2: i8, const MAX_PI_DIV2: i8> fmt::Display
    for AngleNormalizedBase<MIN_PI_DIV2, MAX_PI_DIV2>
{
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{:.5}°", self.to_degrees())
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct AngleNormalizedBaseSerde(f64);
impl<const MIN_PI_DIV2: i8, const MAX_PI_DIV2: i8> TryFrom<AngleNormalizedBaseSerde>
    for AngleNormalizedBase<MIN_PI_DIV2, MAX_PI_DIV2>
{
    type Error = Error;

    fn try_from(value: AngleNormalizedBaseSerde) -> Result<Self, Self::Error> {
        Self::from_radians(value.0)
    }
}
impl<const MIN_PI_DIV2: i8, const MAX_PI_DIV2: i8>
    From<AngleNormalizedBase<MIN_PI_DIV2, MAX_PI_DIV2>> for AngleNormalizedBaseSerde
{
    fn from(value: AngleNormalizedBase<MIN_PI_DIV2, MAX_PI_DIV2>) -> Self {
        AngleNormalizedBaseSerde(value.to_radians())
    }
}
