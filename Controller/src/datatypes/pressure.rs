use anyhow::{ensure, Error};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
#[serde(try_from = "PressureSerde")]
#[serde(into = "PressureSerde")]
pub struct Pressure {
    pascals: f64,
}
impl Pressure {
    pub fn from_pascals(pascals: f64) -> Result<Self, Error> {
        ensure!(pascals.is_finite(), "value must be finite");
        ensure!(pascals >= 0.0, "value must at least zero");
        Ok(Self { pascals })
    }
    pub fn to_pascals(&self) -> f64 {
        self.pascals
    }

    pub fn from_bar(bar: f64) -> Result<Self, Error> {
        ensure!(bar.is_finite(), "value must be finite");
        ensure!(bar >= 0.0, "value must at least zero");
        let pascals = bar * 1e5;
        Ok(Self { pascals })
    }
    pub fn to_bar(&self) -> f64 {
        let bar = self.pascals * 1e-5;
        bar
    }

    pub fn from_millibars_hectopascals(millibars_hectopascals: f64) -> Result<Self, Error> {
        ensure!(millibars_hectopascals.is_finite(), "value must be finite");
        ensure!(millibars_hectopascals >= 0.0, "value must at least zero");
        let pascals = millibars_hectopascals * 1e2;
        Ok(Self { pascals })
    }
    pub fn to_millibars_hectopascals(&self) -> f64 {
        let millibars_hectopascals = self.pascals * 1e-2;
        millibars_hectopascals
    }
}
impl Eq for Pressure {}
#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for Pressure {
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
impl TryFrom<PressureSerde> for Pressure {
    type Error = Error;

    fn try_from(value: PressureSerde) -> Result<Self, Self::Error> {
        Self::from_pascals(value.0)
    }
}
impl Into<PressureSerde> for Pressure {
    fn into(self) -> PressureSerde {
        PressureSerde(self.to_pascals())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct PressureSerde(f64);
