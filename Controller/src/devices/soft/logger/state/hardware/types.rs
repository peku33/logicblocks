use crate::datatypes::{ratio::Ratio, real::Real, temperature::Temperature};
use chrono::{DateTime, Utc};
use std::fmt;

// TODO: Class & Value private

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    Boolean,
    Ratio,
    Real,
    Temperature,
}
impl Class {
    pub fn from_string(input: &str) -> Option<Self> {
        match input {
            "Boolean" => Some(Class::Boolean),
            "Ratio" => Some(Class::Ratio),
            "Real" => Some(Class::Real),
            "Temperature" => Some(Class::Temperature),
            _ => None,
        }
    }
    pub fn to_string(&self) -> &'static str {
        match self {
            Class::Boolean => "Boolean",
            Class::Ratio => "Ratio",
            Class::Real => "Real",
            Class::Temperature => "Temperature",
        }
    }
}

#[derive(Debug)]
pub enum Value {
    Boolean(Option<bool>),
    Ratio(Option<Ratio>),
    Real(Option<Real>),
    Temperature(Option<Temperature>),
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

impl Type for Temperature {
    fn class() -> Class {
        Class::Temperature
    }
    fn into_value(value: Option<Self>) -> Value {
        Value::Temperature(value)
    }
}

#[derive(Debug)]
pub struct TimeValue {
    pub time: DateTime<Utc>,
    pub value: Value,
}
