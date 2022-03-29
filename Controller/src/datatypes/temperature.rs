use anyhow::{ensure, Error};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt};

// TODO: maybe don't use Unit, use from_ and to_ methods

pub enum Unit {
    Kelvin,
    Celsius,
    Fahrenheit,
}

// FIXME: ensure struct field `kelvin` is deserialized in as finite
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Temperature {
    kelvin: f64,
}
impl Temperature {
    pub fn new(
        unit: Unit,
        value: f64,
    ) -> Result<Self, Error> {
        ensure!(value.is_finite(), "value must be finite");
        let kelvin = match unit {
            Unit::Kelvin => value,
            Unit::Fahrenheit => (value + 459.67) * 5.0 / 9.0,
            Unit::Celsius => value + 273.15,
        };
        Ok(Self { kelvin })
    }
    pub fn to_unit(
        self,
        unit: Unit,
    ) -> f64 {
        match unit {
            Unit::Kelvin => self.kelvin,
            Unit::Celsius => self.kelvin - 273.15,
            Unit::Fahrenheit => self.kelvin * 9.0 / 5.0 - 459.67,
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
