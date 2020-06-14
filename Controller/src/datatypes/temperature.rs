use super::DataType;
use std::fmt;

pub enum Unit {
    KELVIN,
    CELSIUS,
    FAHRENHEIT,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Temperature {
    kelvin: f64,
}
impl Temperature {
    pub fn new(
        unit: Unit,
        value: f64,
    ) -> Self {
        let kelvin = match unit {
            Unit::KELVIN => value,
            Unit::FAHRENHEIT => (value + 459.67) * 5.0 / 9.0,
            Unit::CELSIUS => value + 273.15,
        };
        Self { kelvin }
    }
    pub fn to_unit(
        self,
        unit: Unit,
    ) -> f64 {
        match unit {
            Unit::KELVIN => self.kelvin,
            Unit::CELSIUS => self.kelvin - 273.15,
            Unit::FAHRENHEIT => self.kelvin * 9.0 / 5.0 - 459.67,
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
            self.to_unit(Unit::KELVIN),
            self.to_unit(Unit::CELSIUS),
            self.to_unit(Unit::FAHRENHEIT)
        )
    }
}
impl DataType for Temperature {}
