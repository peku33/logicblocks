use super::Base;
use crate::datatypes::{duration::Duration, multiplier::Multiplier};
use std::fmt;

pub trait Value: Base + fmt::Debug {}

impl Value for () {}
impl Value for bool {}

impl Value for Duration {}
impl Value for Multiplier {}
