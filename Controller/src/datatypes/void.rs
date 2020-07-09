use super::DataType;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, Eq, PartialEq, Debug)]
pub struct Void {}
impl Default for Void {
    fn default() -> Self {
        Self {}
    }
}
impl DataType for Void {}