use super::Base;
use crate::datatypes::{ipc_rtsp_url::IpcRtspUrl, temperature::Temperature};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt;

pub trait Value: Base + PartialEq + Serialize + DeserializeOwned + fmt::Debug + 'static {}

impl Value for bool {}
impl Value for Temperature {}
impl Value for IpcRtspUrl {}
