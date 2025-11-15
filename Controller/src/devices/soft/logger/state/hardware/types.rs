use crate::datatypes::{
    current::Current, flow_volumetric::FlowVolumetric, frequency::Frequency,
    multiplier::Multiplier, pressure::Pressure, ratio::Ratio, real::Real, resistance::Resistance,
    temperature::Temperature, voltage::Voltage,
};
use chrono::{DateTime, Utc};
use std::fmt;

// TODO: Class & Value private

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    Boolean,
    Current,
    FlowVolumetric,
    Frequency,
    Multiplier,
    Pressure,
    Ratio,
    Real,
    Resistance,
    Temperature,
    Voltage,
}
impl Class {
    pub fn from_string(input: &str) -> Option<Self> {
        match input {
            "Boolean" => Some(Class::Boolean),
            "Current" => Some(Class::Current),
            "FlowVolumetric" => Some(Class::FlowVolumetric),
            "Frequency" => Some(Class::Frequency),
            "Multiplier" => Some(Class::Multiplier),
            "Pressure" => Some(Class::Pressure),
            "Ratio" => Some(Class::Ratio),
            "Real" => Some(Class::Real),
            "Resistance" => Some(Class::Resistance),
            "Temperature" => Some(Class::Temperature),
            "Voltage" => Some(Class::Voltage),
            _ => None,
        }
    }
    pub fn to_string(&self) -> &'static str {
        match self {
            Class::Boolean => "Boolean",
            Class::Current => "Current",
            Class::FlowVolumetric => "FlowVolumetric",
            Class::Frequency => "Frequency",
            Class::Multiplier => "Multiplier",
            Class::Pressure => "Pressure",
            Class::Ratio => "Ratio",
            Class::Real => "Real",
            Class::Resistance => "Resistance",
            Class::Temperature => "Temperature",
            Class::Voltage => "Voltage",
        }
    }
}

#[derive(Debug)]
pub enum Value {
    Boolean(Option<bool>),
    Current(Option<Current>),
    FlowVolumetric(Option<FlowVolumetric>),
    Frequency(Option<Frequency>),
    Multiplier(Option<Multiplier>),
    Pressure(Option<Pressure>),
    Ratio(Option<Ratio>),
    Real(Option<Real>),
    Resistance(Option<Resistance>),
    Temperature(Option<Temperature>),
    Voltage(Option<Voltage>),
}

pub trait Type: Sized + fmt::Debug + Send + Sync + 'static {
    fn class() -> Class;
    fn into_value(value: Option<Self>) -> Value;
}

impl Type for bool {
    fn class() -> Class {
        Class::Boolean
    }
    fn into_value(value: Option<Self>) -> Value {
        Value::Boolean(value)
    }
}

impl Type for Current {
    fn class() -> Class {
        Class::Current
    }

    fn into_value(value: Option<Self>) -> Value {
        Value::Current(value)
    }
}

impl Type for FlowVolumetric {
    fn class() -> Class {
        Class::FlowVolumetric
    }

    fn into_value(value: Option<Self>) -> Value {
        Value::FlowVolumetric(value)
    }
}

impl Type for Frequency {
    fn class() -> Class {
        Class::Frequency
    }

    fn into_value(value: Option<Self>) -> Value {
        Value::Frequency(value)
    }
}

impl Type for Multiplier {
    fn class() -> Class {
        Class::Multiplier
    }

    fn into_value(value: Option<Self>) -> Value {
        Value::Multiplier(value)
    }
}

impl Type for Pressure {
    fn class() -> Class {
        Class::Pressure
    }

    fn into_value(value: Option<Self>) -> Value {
        Value::Pressure(value)
    }
}

impl Type for Ratio {
    fn class() -> Class {
        Class::Ratio
    }
    fn into_value(value: Option<Self>) -> Value {
        Value::Ratio(value)
    }
}

impl Type for Real {
    fn class() -> Class {
        Class::Real
    }
    fn into_value(value: Option<Self>) -> Value {
        Value::Real(value)
    }
}

impl Type for Resistance {
    fn class() -> Class {
        Class::Resistance
    }

    fn into_value(value: Option<Self>) -> Value {
        Value::Resistance(value)
    }
}

impl Type for Temperature {
    fn class() -> Class {
        Class::Temperature
    }
    fn into_value(value: Option<Self>) -> Value {
        Value::Temperature(value)
    }
}

impl Type for Voltage {
    fn class() -> Class {
        Class::Voltage
    }
    fn into_value(value: Option<Self>) -> Value {
        Value::Voltage(value)
    }
}

#[derive(Debug)]
pub struct TimeValue {
    pub time: DateTime<Utc>,
    pub value: Value,
}
