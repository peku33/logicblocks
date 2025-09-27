use super::Base;
use crate::datatypes::{
    angle::{
        AngleNormalized, AngleNormalizedHalf, AngleNormalizedHalfZeroCentered,
        AngleNormalizedZeroCentered,
    },
    building::window::{WindowOpenStateOpenClosed, WindowOpenStateOpenTiltedClosed},
    color_rgb_boolean::ColorRgbBoolean,
    ipc_rtsp_url::IpcRtspUrl,
    multiplier::Multiplier,
    pressure::Pressure,
    range::Range,
    ratio::Ratio,
    real::Real,
    resistance::Resistance,
    temperature::Temperature,
    voltage::Voltage,
};
use std::fmt;

pub trait Value: Base + Eq + fmt::Debug + 'static {}

//
impl Value for bool {}

// datatypes
impl Value for AngleNormalized {}
impl Value for AngleNormalizedHalf {}
impl Value for AngleNormalizedHalfZeroCentered {}
impl Value for AngleNormalizedZeroCentered {}
impl Value for ColorRgbBoolean {}
impl Value for IpcRtspUrl {}
impl Value for Multiplier {}
impl Value for Pressure {}
impl Value for Ratio {}
impl Value for Real {}
impl Value for Resistance {}
impl Value for Temperature {}
impl Value for Voltage {}

// datatypes parent
impl<V> Value for Range<V> where V: Value {}

// datatypes::building
impl Value for WindowOpenStateOpenClosed {}
impl Value for WindowOpenStateOpenTiltedClosed {}
