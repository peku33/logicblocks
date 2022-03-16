use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub struct ColorRgbBoolean {
    pub r: bool,
    pub g: bool,
    pub b: bool,
}
impl ColorRgbBoolean {
    pub const fn off() -> Self {
        Self {
            r: false,
            g: false,
            b: false,
        }
    }
}
