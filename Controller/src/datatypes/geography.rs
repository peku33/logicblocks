use anyhow::{Context, Error, Ok, ensure};
use serde::{Deserialize, Serialize};
use std::fmt;
use typed_floats::{NonNaNFinite, PositiveFinite};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
#[serde(try_from = "LatitudeSerde")]
#[serde(into = "LatitudeSerde")]
pub struct Latitude {
    // positive north, negative south
    radians: NonNaNFinite<f64>,
}
impl Latitude {
    pub fn from_radians(radians: f64) -> Result<Self, Error> {
        let radians = NonNaNFinite::<f64>::new(radians)?;
        ensure!(
            (-(std::f64::consts::PI / 2.0)..=(std::f64::consts::PI / 2.0)).contains(&radians),
            "value must be in range [-pi/2, pi/2]"
        );

        Ok(Self { radians })
    }
    pub fn to_radians(&self) -> f64 {
        self.radians.get()
    }

    pub fn from_degrees(degrees: f64) -> Result<Self, Error> {
        ensure!(
            (-90.0..=90.0).contains(&degrees),
            "value must be from -90.0 to 90.0"
        );
        let radians = degrees.to_radians();

        let self_ = Self::from_radians(radians).context("from_radians")?;

        Ok(self_)
    }
    pub fn to_degrees(&self) -> f64 {
        self.to_radians().to_degrees()
    }

    pub fn from_degrees_n(degrees: f64) -> Result<Self, Error> {
        ensure!(
            (0.0..=90.0).contains(&degrees),
            "value must be from 0.0 to 90.0"
        );
        let radians = degrees.to_radians();

        let self_ = Self::from_radians(radians).context("from_radians")?;

        Ok(self_)
    }
    pub fn from_degrees_s(degrees: f64) -> Result<Self, Error> {
        ensure!(
            (0.0..=90.0).contains(&degrees),
            "value must be from 0.0 to 90.0"
        );
        let radians = -degrees.to_radians();

        let self_ = Self::from_radians(radians).context("from_radians")?;

        Ok(self_)
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
            self.to_degrees().abs(),
            if self.to_radians() >= 0.0 { 'N' } else { 'S' }
        )
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct LatitudeSerde(f64);
impl TryFrom<LatitudeSerde> for Latitude {
    type Error = Error;

    fn try_from(value: LatitudeSerde) -> Result<Self, Self::Error> {
        Self::from_radians(value.0)
    }
}
impl From<Latitude> for LatitudeSerde {
    fn from(value: Latitude) -> Self {
        LatitudeSerde(value.to_radians())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
#[serde(try_from = "LongitudeSerde")]
#[serde(into = "LongitudeSerde")]
pub struct Longitude {
    // positive east
    // negative west
    radians: NonNaNFinite<f64>,
}
impl Longitude {
    pub fn from_radians(radians: f64) -> Result<Self, Error> {
        let radians = NonNaNFinite::<f64>::new(radians)?;
        ensure!(
            (-(std::f64::consts::PI)..=(std::f64::consts::PI)).contains(&radians),
            "value must be in range [-pi, pi]"
        );

        Ok(Self { radians })
    }
    pub fn to_radians(&self) -> f64 {
        self.radians.get()
    }

    pub fn from_degrees(degrees: f64) -> Result<Self, Error> {
        ensure!(
            (-180.0..=180.0).contains(&degrees),
            "value must be from -180.0 to 180.0"
        );
        let radians = degrees.to_radians();

        let self_ = Self::from_radians(radians).context("from_radians")?;

        Ok(self_)
    }
    pub fn to_degrees(&self) -> f64 {
        self.to_radians().to_degrees()
    }

    pub fn from_degrees_e(degrees: f64) -> Result<Self, Error> {
        ensure!(
            (0.0..=180.0).contains(&degrees),
            "value must be from 0.0 to 180.0"
        );
        let radians = degrees.to_radians();

        let self_ = Self::from_radians(radians).context("from_radians")?;

        Ok(self_)
    }
    pub fn from_degrees_w(degrees: f64) -> Result<Self, Error> {
        ensure!(
            (0.0..=180.0).contains(&degrees),
            "value must be from 0.0 to 180.0"
        );
        let radians = -degrees.to_radians();

        let self_ = Self::from_radians(radians).context("from_radians")?;

        Ok(self_)
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
            self.to_degrees().abs(),
            if self.to_radians() >= 0.0 { 'E' } else { 'W' }
        )
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct LongitudeSerde(f64);
impl TryFrom<LongitudeSerde> for Longitude {
    type Error = Error;

    fn try_from(value: LongitudeSerde) -> Result<Self, Self::Error> {
        Self::from_radians(value.0)
    }
}
impl From<Longitude> for LongitudeSerde {
    fn from(value: Longitude) -> Self {
        LongitudeSerde(value.to_radians())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Elevation {
    meters: PositiveFinite<f64>,
}
impl Elevation {
    pub fn from_meters(meters: f64) -> Result<Self, Error> {
        let meters = PositiveFinite::<f64>::new(meters)?;

        Ok(Self { meters })
    }
    pub fn to_meters(&self) -> f64 {
        self.meters.get()
    }
}
impl fmt::Display for Elevation {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{:.2}m", self.to_meters())
    }
}

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
