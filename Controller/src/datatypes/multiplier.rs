use super::real::Real;
use anyhow::Error;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    iter::Sum,
    ops::{Add, AddAssign, Sub, SubAssign},
};
use typed_floats::PositiveFinite;

// NOTE: will panic for Add/AddAssign going infinity and Sub/SubAssign going
// below zero
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Multiplier(PositiveFinite<f64>);
impl Multiplier {
    pub const fn zero() -> Self {
        // SAFE: 0.0 is positive and finite
        let inner = unsafe { PositiveFinite::<f64>::new_unchecked(0f64) };

        Self(inner)
    }
    pub const fn one() -> Self {
        // SAFE: 1.0 is positive and finite
        let inner = unsafe { PositiveFinite::<f64>::new_unchecked(1f64) };

        Self(inner)
    }

    pub fn from_f64(value: f64) -> Result<Self, Error> {
        let value = PositiveFinite::<f64>::new(value)?;

        Ok(Self(value))
    }
    pub fn to_f64(&self) -> f64 {
        self.0.get()
    }
}
impl Add for Multiplier {
    type Output = Self;

    fn add(
        self,
        rhs: Self,
    ) -> Self::Output {
        Self::from_f64(self.to_f64() + rhs.to_f64()).unwrap()
    }
}
impl AddAssign for Multiplier {
    fn add_assign(
        &mut self,
        rhs: Self,
    ) {
        *self = Self::from_f64(self.to_f64() + rhs.to_f64()).unwrap()
    }
}
impl Sub for Multiplier {
    type Output = Self;

    fn sub(
        self,
        rhs: Self,
    ) -> Self::Output {
        Self::from_f64(self.to_f64() - rhs.to_f64()).unwrap()
    }
}
impl SubAssign for Multiplier {
    fn sub_assign(
        &mut self,
        rhs: Self,
    ) {
        *self = Self::from_f64(self.to_f64() - rhs.to_f64()).unwrap()
    }
}
impl Sum for Multiplier {
    fn sum<I: Iterator<Item = Self>>(iterator: I) -> Self {
        Self::from_f64(iterator.map(|item| item.to_f64()).sum()).unwrap()
    }
}
impl fmt::Display for Multiplier {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{:.2}x", self.to_f64())
    }
}

impl TryFrom<Real> for Multiplier {
    type Error = Error;

    fn try_from(value: Real) -> Result<Self, Self::Error> {
        Self::from_f64(value.to_f64())
    }
}
impl From<Multiplier> for Real {
    fn from(value: Multiplier) -> Self {
        Self::from_f64(value.to_f64()).unwrap()
    }
}
