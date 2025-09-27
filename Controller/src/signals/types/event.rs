use super::Base;
use crate::datatypes::multiplier::Multiplier;
use std::{fmt, time::Duration};

pub trait Value: Base + fmt::Debug {}

// std types
impl Value for () {}
impl Value for bool {}
impl Value for Duration {}

// datatypes
impl Value for Multiplier {}
