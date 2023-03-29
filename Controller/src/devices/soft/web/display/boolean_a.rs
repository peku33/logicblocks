use super::common::state_a;
use std::borrow::Cow;

#[derive(Debug)]
pub struct Specification {}
impl state_a::Specification for Specification {
    type Type = bool;

    fn name() -> std::borrow::Cow<'static, str> {
        Cow::from("boolean_a")
    }
}

pub type Device = state_a::Device<Specification>;
pub type SignalIdentifier = state_a::SignalIdentifier;
