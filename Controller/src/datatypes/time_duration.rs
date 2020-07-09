use super::DataType;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Copy, Clone, Eq, PartialEq, Debug)]
pub struct TimeDuration {
    value: Duration,
}
impl From<Duration> for TimeDuration {
    fn from(value: Duration) -> Self {
        Self { value }
    }
}
impl Into<Duration> for TimeDuration {
    fn into(self) -> Duration {
        self.value
    }
}
impl DataType for TimeDuration {}
