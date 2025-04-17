use anyhow::{Error, ensure};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
#[serde(try_from = "ResistanceSerde")]
#[serde(into = "ResistanceSerde")]
pub struct Resistance {
    ohms: f64,
}
impl Resistance {
    pub const fn zero() -> Self {
        Self { ohms: 0.0 }
    }
    pub const fn infinity() -> Self {
        Self {
            ohms: f64::INFINITY,
        }
    }

    pub fn from_ohms(ohms: f64) -> Result<Self, Error> {
        ensure!(
            ohms.is_finite() || ohms == f64::INFINITY,
            "ohms must be [0.0 to +InF]"
        );
        Ok(Self { ohms })
    }
    pub fn to_ohms(&self) -> f64 {
        self.ohms
    }
}
impl Eq for Resistance {}
#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for Resistance {
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct ResistanceSerde(f64);
impl TryFrom<ResistanceSerde> for Resistance {
    type Error = Error;

    fn try_from(value: ResistanceSerde) -> Result<Self, Self::Error> {
        Resistance::from_ohms(value.0)
    }
}
impl From<Resistance> for ResistanceSerde {
    fn from(value: Resistance) -> Self {
        Self(value.to_ohms())
    }
}
