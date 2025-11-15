use super::common::state_a as parent;
use std::borrow::Cow;

#[derive(Debug)]
pub struct Specification;
impl parent::Specification for Specification {
    type Type = bool;

    fn name() -> std::borrow::Cow<'static, str> {
        Cow::from("boolean_a")
    }
}

pub type Device = parent::Device<Specification>;
pub type SignalIdentifier = parent::SignalIdentifier;
