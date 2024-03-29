use serde::Serialize;

#[derive(Clone, Copy, Serialize, Debug)]
#[serde(into = "IconPrefixString")]
pub enum IconPrefix {
    Solid,
    Regular,
}
#[derive(Debug, Serialize)]
#[serde(transparent)]
struct IconPrefixString {
    inner: &'static str,
}
impl From<IconPrefix> for IconPrefixString {
    fn from(from: IconPrefix) -> Self {
        let inner = match from {
            IconPrefix::Solid => "fas",
            IconPrefix::Regular => "far",
        };
        Self { inner }
    }
}

#[derive(Clone, Serialize, Debug)]
pub struct Icon {
    pub prefix: IconPrefix,
    pub name: String,
}
