use super::real::Real;
use anyhow::{Error, ensure};
use derive_more::{Add, AddAssign};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt};

#[derive(Clone, Copy, PartialEq, PartialOrd, Add, AddAssign, Debug, Serialize, Deserialize)]
#[serde(try_from = "DurationSerde")]
#[serde(into = "DurationSerde")]
pub struct Duration {
    seconds: f64,
}
impl Duration {
    pub const fn zero() -> Self {
        Self { seconds: 0.0 }
    }

    pub fn from_seconds(seconds: f64) -> Result<Self, Error> {
        ensure!(seconds.is_finite(), "value must be finite");
        ensure!(seconds >= 0.0, "value must be positive");
        Ok(Self { seconds })
    }
    pub fn to_seconds(&self) -> f64 {
        self.seconds
    }

    pub fn from_std(std: std::time::Duration) -> Self {
        let seconds = std.as_secs_f64();
        Self { seconds }
    }
    pub fn to_std(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f64(self.seconds)
    }
}
impl Eq for Duration {}
#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for Duration {
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
impl fmt::Display for Duration {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{:.3}s", self.seconds)
    }
}

impl TryFrom<Real> for Duration {
    type Error = Error;

    fn try_from(value: Real) -> Result<Self, Self::Error> {
        Self::from_seconds(value.to_f64())
    }
}
impl From<Duration> for Real {
    fn from(value: Duration) -> Self {
        Self::from_f64(value.to_seconds()).unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct DurationSerde(f64);
impl TryFrom<DurationSerde> for Duration {
    type Error = Error;

    fn try_from(value: DurationSerde) -> Result<Self, Self::Error> {
        Self::from_seconds(value.0)
    }
}
impl From<Duration> for DurationSerde {
    fn from(value: Duration) -> Self {
        DurationSerde(value.to_seconds())
    }
}
