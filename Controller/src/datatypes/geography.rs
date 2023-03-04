use anyhow::{ensure, Error, Ok};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt};

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
#[serde(try_from = "LatitudeSerde")]
#[serde(into = "LatitudeSerde")]
pub struct Latitude {
    // positive north, negative south
    radians: f64,
}
impl Latitude {
    pub fn from_radians(radians: f64) -> Result<Self, Error> {
        ensure!(radians.is_finite(), "value must be finite");
        ensure!(
            (-(std::f64::consts::PI / 2.0)..=(std::f64::consts::PI / 2.0)).contains(&radians),
            "value must be in range [-pi/2, pi/2]"
        );
        Ok(Self { radians })
    }
    pub fn to_radians(&self) -> f64 {
        self.radians
    }

    pub fn from_degrees(degrees: f64) -> Result<Self, Error> {
        ensure!(
            (-90.0..=90.0).contains(&degrees),
            "value must be from -90.0 to 90.0"
        );
        let radians = degrees.to_radians();
        Ok(Self { radians })
    }
    pub fn from_degrees_n(degrees: f64) -> Result<Self, Error> {
        ensure!(
            (0.0..=90.0).contains(&degrees),
            "value must be from 0.0 to 90.0"
        );
        let radians = degrees.to_radians();
        Ok(Self { radians })
    }
    pub fn from_degrees_s(degrees: f64) -> Result<Self, Error> {
        ensure!(
            (0.0..=90.0).contains(&degrees),
            "value must be from 0.0 to 90.0"
        );
        let radians = -degrees.to_radians();
        Ok(Self { radians })
    }
}
impl Eq for Latitude {}
#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for Latitude {
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
impl TryFrom<LatitudeSerde> for Latitude {
    type Error = Error;

    fn try_from(value: LatitudeSerde) -> Result<Self, Self::Error> {
        Self::from_radians(value.0)
    }
}
impl Into<LatitudeSerde> for Latitude {
    fn into(self) -> LatitudeSerde {
        LatitudeSerde(self.to_radians())
    }
}
impl fmt::Display for Latitude {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "{:.5}° {}",
            self.radians.to_degrees().abs(),
            if self.radians >= 0.0 { 'N' } else { 'S' }
        )
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct LatitudeSerde(f64);

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
#[serde(try_from = "LongitudeSerde")]
#[serde(into = "LongitudeSerde")]
pub struct Longitude {
    // positive east
    // negative west
    radians: f64,
}
impl Longitude {
    pub fn from_radians(radians: f64) -> Result<Self, Error> {
        ensure!(radians.is_finite(), "value must be finite");
        ensure!(
            (-(std::f64::consts::PI)..=(std::f64::consts::PI)).contains(&radians),
            "value must be in range [-pi, pi]"
        );
        Ok(Self { radians })
    }
    pub fn to_radians(&self) -> f64 {
        self.radians
    }

    pub fn from_degrees(degrees: f64) -> Result<Self, Error> {
        ensure!(
            (-180.0..=180.0).contains(&degrees),
            "value must be from -180.0 to 180.0"
        );
        let radians = degrees.to_radians();
        Ok(Self { radians })
    }
    pub fn from_degrees_e(degrees: f64) -> Result<Self, Error> {
        ensure!(
            (0.0..=180.0).contains(&degrees),
            "value must be from 0.0 to 180.0"
        );
        let radians = degrees.to_radians();
        Ok(Self { radians })
    }
    pub fn from_degrees_w(degrees: f64) -> Result<Self, Error> {
        ensure!(
            (0.0..=180.0).contains(&degrees),
            "value must be from 0.0 to 180.0"
        );
        let radians = -degrees.to_radians();
        Ok(Self { radians })
    }
}
impl Eq for Longitude {}
#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for Longitude {
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
impl TryFrom<LongitudeSerde> for Longitude {
    type Error = Error;

    fn try_from(value: LongitudeSerde) -> Result<Self, Self::Error> {
        Self::from_radians(value.0)
    }
}
impl Into<LongitudeSerde> for Longitude {
    fn into(self) -> LongitudeSerde {
        LongitudeSerde(self.to_radians())
    }
}
impl fmt::Display for Longitude {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "{:.5}° {}",
            self.radians.to_degrees().abs(),
            if self.radians >= 0.0 { 'E' } else { 'W' }
        )
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct LongitudeSerde(f64);

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
#[serde(try_from = "ElevationSerde")]
#[serde(into = "ElevationSerde")]
pub struct Elevation {
    meters: f64,
}
impl Elevation {
    pub fn from_meters(meters: f64) -> Result<Self, Error> {
        ensure!(meters.is_finite(), "value must be finite");
        ensure!(meters >= 0.0, "value must be positive");
        Ok(Self { meters })
    }
    pub fn to_meters(&self) -> f64 {
        self.meters
    }
}
impl Eq for Elevation {}
#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for Elevation {
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
impl TryFrom<ElevationSerde> for Elevation {
    type Error = Error;

    fn try_from(value: ElevationSerde) -> Result<Self, Self::Error> {
        Self::from_meters(value.0)
    }
}
impl Into<ElevationSerde> for Elevation {
    fn into(self) -> ElevationSerde {
        ElevationSerde(self.to_meters())
    }
}
impl fmt::Display for Elevation {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{:.2} m", self.meters)
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct ElevationSerde(f64);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct Coordinates2d {
    pub latitude: Latitude,
    pub longitude: Longitude,
}
impl fmt::Display for Coordinates2d {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}, {}", self.latitude, self.longitude)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct Coordinates3d {
    pub coordinates_2d: Coordinates2d,
    pub elevation: Elevation,
}
impl fmt::Display for Coordinates3d {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}, {}", self.coordinates_2d, self.elevation)
    }
}
