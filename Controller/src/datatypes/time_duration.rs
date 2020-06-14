use super::DataType;
use std::time::Duration;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
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
