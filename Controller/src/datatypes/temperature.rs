use serde::{Deserialize, Serialize};
use std::fmt;

pub enum Unit {
    Kelvin,
    Celsius,
    Fahrenheit,
}

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Debug)]
#[serde(transparent)]
pub struct Temperature {
    kelvin: f64,
}
impl Temperature {
    pub fn new(
        unit: Unit,
        value: f64,
    ) -> Self {
        let kelvin = match unit {
            Unit::Kelvin => value,
            Unit::Fahrenheit => (value + 459.67) * 5.0 / 9.0,
            Unit::Celsius => value + 273.15,
        };
        Self { kelvin }
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
