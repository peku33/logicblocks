use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct RangeBoundary<T> {
    pub value: T,
    pub inclusive: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct Range<T> {
    pub lower: Option<RangeBoundary<T>>,
    pub upper: Option<RangeBoundary<T>>,
}
impl<T> Range<T>
where
    T: PartialOrd,
{
    pub fn contains(
        &self,
        value: &T,
    ) -> bool {
        if let Some(lower) = &self.lower {
            if lower.inclusive && *value == lower.value {
                return true;
            }
            if *value < lower.value {
                return false;
            }
        }
        if let Some(upper) = &self.upper {
            if upper.inclusive && *value == upper.value {
                return true;
            }
            if *value > upper.value {
                return false;
            }
        }

        true
    }
}
impl<T> Range<T>
where
    T: Clone + PartialOrd,
{
    pub fn clamp_to(
        &self,
        value: T,
    ) -> T {
        // TODO: what if self.lower > self.upper?
        if let Some(lower) = &self.lower
            && value < lower.value
        {
            return lower.value.clone();
        }
        if let Some(upper) = &self.upper
            && value > upper.value
        {
            return upper.value.clone();
        }

        value
    }
}
