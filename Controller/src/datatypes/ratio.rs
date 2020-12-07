use anyhow::{ensure, Error};

#[derive(PartialOrd, PartialEq, Copy, Clone, Debug)]
pub struct Ratio {
    value: f64,
}
impl Ratio {
    pub fn new(value: f64) -> Result<Self, Error> {
        ensure!(value.is_finite(), "value must be finite");
        ensure!(
            (0.0..=1.0).contains(&value),
            "value must be between 0.0 and 1.0"
        );
        Ok(Self { value })
    }
    pub const fn epsilon() -> Self {
        Self {
            value: f64::EPSILON,
        }
    }

    pub fn as_f64(&self) -> f64 {
        self.value
    }
}
