use super::common::state_a as parent;
use crate::datatypes::pressure::Pressure;
use std::borrow::Cow;

#[derive(Debug)]
pub struct Specification;
impl parent::Specification for Specification {
    type Type = Pressure;

    fn name() -> std::borrow::Cow<'static, str> {
        Cow::from("pressure_a")
    }
}

pub type Device = parent::Device<Specification>;
pub type SignalIdentifier = parent::SignalIdentifier;
