use super::Base;
use crate::datatypes::multiplier::Multiplier;
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt, time::Duration};

pub trait Value: Base + Serialize + DeserializeOwned + fmt::Debug {}

impl Value for () {}
impl Value for bool {}

impl Value for Duration {}
impl Value for Multiplier {}
