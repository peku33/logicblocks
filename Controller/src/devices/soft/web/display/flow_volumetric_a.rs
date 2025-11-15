use super::common::state_a as parent;
use crate::datatypes::flow_volumetric::FlowVolumetric;
use std::borrow::Cow;

#[derive(Debug)]
pub struct Specification;
impl parent::Specification for Specification {
    type Type = FlowVolumetric;

    fn name() -> std::borrow::Cow<'static, str> {
        Cow::from("flow_volumetric_a")
    }
}

pub type Device = parent::Device<Specification>;
pub type SignalIdentifier = parent::SignalIdentifier;
