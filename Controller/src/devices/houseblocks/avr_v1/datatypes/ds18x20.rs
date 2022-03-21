use crate::{datatypes::temperature::Temperature, signals};
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum SensorType {
    Empty,
    Invalid,
    S,
    B,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub struct State {
    pub sensor_type: SensorType,
    pub reset_count: u8,
    pub temperature: Option<Temperature>,
}
impl signals::types::state::Value for State {}
