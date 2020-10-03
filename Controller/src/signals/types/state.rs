use super::Base;
use crate::datatypes::temperature::Temperature;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt;

pub trait Value: Base + PartialEq + Serialize + DeserializeOwned + fmt::Debug + 'static {}

impl Value for bool {}
impl Value for Option<bool> {}

impl Value for Temperature {}
impl Value for Option<Temperature> {}
