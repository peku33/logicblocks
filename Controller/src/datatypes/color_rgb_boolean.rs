#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ColorRgbBoolean {
    pub r: bool,
    pub g: bool,
    pub b: bool,
}
impl ColorRgbBoolean {
    pub fn off() -> Self {
        Self {
            r: false,
            g: false,
            b: false,
        }
    }
}
