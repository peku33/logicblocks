use super::super::common::state_a;
use crate::datatypes::building::window::WindowOpenStateOpenTiltedClosed;
use std::borrow::Cow;

#[derive(Debug)]
pub struct Specification {}
impl state_a::Specification for Specification {
    type Type = WindowOpenStateOpenTiltedClosed;

    fn name() -> std::borrow::Cow<'static, str> {
        Cow::from("building/window_open_state_open_tilted_closed_a")
    }
}

pub type Device = state_a::Device<Specification>;
pub type SignalIdentifier = state_a::SignalIdentifier;
