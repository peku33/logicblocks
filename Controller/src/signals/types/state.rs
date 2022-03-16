use super::Base;
use crate::datatypes::{
    color_rgb_boolean::ColorRgbBoolean, ipc_rtsp_url::IpcRtspUrl, multiplier::Multiplier,
    ratio::Ratio, real::Real, temperature::Temperature, voltage::Voltage,
};
use std::fmt;

pub trait Value: Base + Eq + fmt::Debug + 'static {}

impl Value for bool {}

impl Value for ColorRgbBoolean {}
impl Value for IpcRtspUrl {}
impl Value for Multiplier {}
impl Value for Ratio {}
impl Value for Real {}
impl Value for Temperature {}
impl Value for Voltage {}
