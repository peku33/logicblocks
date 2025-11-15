use anyhow::Error;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign},
};
use typed_floats::NonNaNFinite;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Real(NonNaNFinite<f64>);
impl Real {
    pub const fn zero() -> Self {
        // SAFE: 0.0 is always finite
        let inner = unsafe { NonNaNFinite::<f64>::new_unchecked(0f64) };

        Self(inner)
    }

    pub fn from_f64(value: f64) -> Result<Self, Error> {
        let value = NonNaNFinite::<f64>::new(value)?;

        Ok(Self(value))
    }
    pub fn to_f64(&self) -> f64 {
        self.0.get()
    }
}
impl Add for Real {
    type Output = Self;

    fn add(
        self,
        rhs: Self,
    ) -> Self::Output {
        Self::from_f64(self.to_f64() + rhs.to_f64()).unwrap()
    }
}
impl AddAssign for Real {
    fn add_assign(
        &mut self,
        rhs: Self,
    ) {
        *self = Self::from_f64(self.to_f64() + rhs.to_f64()).unwrap()
    }
}
impl Sub for Real {
    type Output = Self;

    fn sub(
        self,
        rhs: Self,
    ) -> Self::Output {
        Self::from_f64(self.to_f64() - rhs.to_f64()).unwrap()
    }
}
impl SubAssign for Real {
    fn sub_assign(
        &mut self,
        rhs: Self,
    ) {
        *self = Self::from_f64(self.to_f64() - rhs.to_f64()).unwrap()
    }
}
impl Mul for Real {
    type Output = Self;

    fn mul(
        self,
        rhs: Self,
    ) -> Self::Output {
        Self::from_f64(self.to_f64() * rhs.to_f64()).unwrap()
    }
}
impl MulAssign for Real {
    fn mul_assign(
        &mut self,
        rhs: Self,
    ) {
        *self = Self::from_f64(self.to_f64() * rhs.to_f64()).unwrap()
    }
}
impl fmt::Display for Real {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.to_f64())
    }
}

// From/To for external types
impl TryFrom<Real> for std::time::Duration {
    type Error = Error;

    fn try_from(value: Real) -> Result<Self, Self::Error> {
        Self::try_from_secs_f64(value.to_f64()).map_err(|error| error.into())
    }
}
impl TryFrom<std::time::Duration> for Real {
    type Error = Error;

    fn try_from(value: std::time::Duration) -> Result<Self, Self::Error> {
        Self::from_f64(value.as_secs_f64())
    }
}
