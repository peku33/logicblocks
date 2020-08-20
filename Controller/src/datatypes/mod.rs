pub mod boolean;
pub mod temperature;
pub mod time_duration;
pub mod void;

use serde::{Deserialize, Serialize};

pub trait DataType: Serialize + Deserialize<'static> {}

impl<T> DataType for Option<T> where T: DataType {}
