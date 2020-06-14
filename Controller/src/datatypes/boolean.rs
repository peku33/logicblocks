use super::DataType;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Boolean {
    value: bool,
}
impl From<bool> for Boolean {
    fn from(value: bool) -> Self {
        Self { value }
    }
}
impl Into<bool> for Boolean {
    fn into(self) -> bool {
        self.value
    }
}
impl DataType for Boolean {}
