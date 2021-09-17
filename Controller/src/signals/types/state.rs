use super::Base;
use crate::datatypes::{
    ipc_rtsp_url::IpcRtspUrl, multiplier::Multiplier, ratio::Ratio, temperature::Temperature,
};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt;

pub trait Value:
    Base + PartialEq + Eq + Serialize + DeserializeOwned + fmt::Debug + 'static
{
}

impl Value for bool {}

impl Value for IpcRtspUrl {}
impl Value for Multiplier {}
impl Value for Ratio {}
impl Value for Temperature {}
