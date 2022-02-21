use anyhow::{Context, Error};
use http::uri::Uri;
use serde::{Deserialize, Serialize};
use std::{fmt, fmt::Display, str::FromStr};

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(try_from = "String")]
#[serde(into = "String")]
pub struct IpcRtspUrl(pub Uri);
impl Display for IpcRtspUrl {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(self.0.to_string().as_str())
    }
}
impl FromStr for IpcRtspUrl {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let uri: Uri = value.parse().context("parse")?;
        Ok(Self(uri))
    }
}
impl TryFrom<String> for IpcRtspUrl {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl Into<String> for IpcRtspUrl {
    fn into(self) -> String {
        self.to_string()
    }
}
