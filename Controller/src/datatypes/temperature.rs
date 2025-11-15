use super::real::Real;
use anyhow::{Context, Error};
use serde::{Deserialize, Serialize};
use std::fmt;
use typed_floats::NonNaNFinite;

#[derive(Debug)]
pub enum Unit {
    Kelvin,
    Celsius,
    Fahrenheit,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Temperature {
    kelvins: NonNaNFinite<f64>,
}
impl Temperature {
    pub fn from_kelvins(kelvins: f64) -> Result<Self, Error> {
        let kelvins = NonNaNFinite::<f64>::new(kelvins)?;

        Ok(Self { kelvins })
    }
    pub fn to_kelvins(&self) -> f64 {
        self.kelvins.get()
    }

    pub fn from_unit(
        unit: Unit,
        value: f64,
    ) -> Result<Self, Error> {
        let kelvins = match unit {
            Unit::Kelvin => value,
            Unit::Fahrenheit => (value + 459.67) * 5.0 / 9.0,
            Unit::Celsius => value + 273.15,
        };

        let self_ = Self::from_kelvins(kelvins).context("from_kelvins")?;

        Ok(self_)
    }
    pub fn to_unit(
        self,
        unit: Unit,
    ) -> f64 {
        let kelvins = self.to_kelvins();

        let value = match unit {
            Unit::Kelvin => kelvins,
            Unit::Celsius => kelvins - 273.15,
            Unit::Fahrenheit => kelvins * 9.0 / 5.0 - 459.67,
        };

        value
    }
}
impl fmt::Display for Temperature {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "{}*K / {}*C / {}*F",
            self.to_unit(Unit::Kelvin),
            self.to_unit(Unit::Celsius),
            self.to_unit(Unit::Fahrenheit)
        )
    }
}

impl TryFrom<Real> for Temperature {
    type Error = Error;

    fn try_from(value: Real) -> Result<Self, Self::Error> {
        Self::from_kelvins(value.to_f64())
    }
}
impl From<Temperature> for Real {
    fn from(value: Temperature) -> Self {
        Self::from_f64(value.to_kelvins()).unwrap()
    }
}
