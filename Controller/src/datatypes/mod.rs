pub mod boolean;
pub mod temperature;
pub mod time_duration;
pub mod void;

pub trait DataType {}

impl<T> DataType for Option<T> where T: DataType {}
