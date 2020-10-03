pub mod event;
pub mod state;

use std::any::Any;

pub trait Base = Any + Send + Sync + 'static;
