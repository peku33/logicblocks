use anyhow::{Error, ensure};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
#[serde(try_from = "VoltageSerde")]
#[serde(into = "VoltageSerde")]
pub struct Voltage {
    volts: f64,
}
impl Voltage {
    pub fn from_volts(volts: f64) -> Result<Self, Error> {
        ensure!(volts.is_finite(), "volts must be finite");
        Ok(Self { volts })
    }
    pub fn to_volts(&self) -> f64 {
        self.volts
    }
}
impl Eq for Voltage {}
#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for Voltage {
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct VoltageSerde(f64);
impl TryFrom<VoltageSerde> for Voltage {
    type Error = Error;

    fn try_from(value: VoltageSerde) -> Result<Self, Self::Error> {
        Voltage::from_volts(value.0)
    }
}
impl From<Voltage> for VoltageSerde {
    fn from(value: Voltage) -> Self {
        Self(value.to_volts())
    }
}
