use anyhow::Error;
use http::uri::Uri;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
#[serde(try_from = "IpcRtspUrlSerde")]
#[serde(into = "IpcRtspUrlSerde")]
pub struct IpcRtspUrl {
    uri: Uri,
}
impl IpcRtspUrl {
    pub fn new(uri: Uri) -> Self {
        Self { uri }
    }
}
impl TryFrom<IpcRtspUrlSerde> for IpcRtspUrl {
    type Error = Error;

    fn try_from(value: IpcRtspUrlSerde) -> Result<Self, Self::Error> {
        let uri: Uri = value.uri.parse()?;
        Ok(Self { uri })
    }
}
impl Into<IpcRtspUrlSerde> for IpcRtspUrl {
    fn into(self) -> IpcRtspUrlSerde {
        IpcRtspUrlSerde {
            uri: self.uri.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct IpcRtspUrlSerde {
    uri: String,
}
