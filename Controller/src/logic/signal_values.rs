use super::signal::{EventValue, StateValue, Value};

#[derive(Debug)]
pub struct Void {}
impl Void {
    pub fn new() -> Self {
        Self {}
    }
}
impl Value for Void {}
impl EventValue for Void {}

#[derive(PartialEq, Eq, Debug)]
pub struct Bool {
    value: bool,
}
impl Bool {
    pub fn new(value: bool) -> Self {
        Self { value }
    }
    pub fn value(&self) -> bool {
        self.value
    }
}
impl Value for Bool {}
impl StateValue for Bool {}
