use super::Base;
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt, time::Duration};

pub trait Value: Base + Serialize + DeserializeOwned + fmt::Debug {}

impl Value for () {}
impl Value for Duration {}
