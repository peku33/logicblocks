use super::real::Real;
use anyhow::{Error, ensure};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt};
#[derive(Debug)]
pub enum Unit {
    Kelvin,
    Celsius,
    Fahrenheit,
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
#[serde(try_from = "TemperatureSerde")]
#[serde(into = "TemperatureSerde")]
pub struct Temperature {
    kelvins: f64,
}
impl Temperature {
    pub fn from_kelvins(kelvins: f64) -> Result<Self, Error> {
        ensure!(kelvins.is_finite(), "value must be finite");
        Ok(Self { kelvins })
    }
    pub fn to_kelvins(&self) -> f64 {
        self.kelvins
    }

    pub fn from_unit(
        unit: Unit,
        value: f64,
    ) -> Result<Self, Error> {
        ensure!(value.is_finite(), "value must be finite");
        let kelvin = match unit {
            Unit::Kelvin => value,
            Unit::Fahrenheit => (value + 459.67) * 5.0 / 9.0,
            Unit::Celsius => value + 273.15,
        };
        Ok(Self { kelvins: kelvin })
    }
    pub fn to_unit(
        self,
        unit: Unit,
    ) -> f64 {
        match unit {
            Unit::Kelvin => self.kelvins,
            Unit::Celsius => self.kelvins - 273.15,
            Unit::Fahrenheit => self.kelvins * 9.0 / 5.0 - 459.67,
        }
    }
}
impl Eq for Temperature {}
#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for Temperature {
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        self.partial_cmp(other).unwrap()
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct TemperatureSerde(f64);
impl TryFrom<TemperatureSerde> for Temperature {
    type Error = Error;

    fn try_from(value: TemperatureSerde) -> Result<Self, Self::Error> {
        Self::from_kelvins(value.0)
    }
}
impl From<Temperature> for TemperatureSerde {
    fn from(value: Temperature) -> Self {
        TemperatureSerde(value.to_kelvins())
    }
}
