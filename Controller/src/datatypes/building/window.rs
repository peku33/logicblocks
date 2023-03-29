use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum WindowOpenStateOpenClosed {
    Open,
    Closed,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum WindowOpenStateOpenTiltedClosed {
    Open,
    Tilted,
    Closed,
}
