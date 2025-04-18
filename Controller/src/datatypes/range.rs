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
    T: Ord,
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

    pub fn clamp<'s, 'v: 's>(
        &'s self,
        value: &'v T,
    ) -> &'s T {
        // TODO: what is self.lower > self.upper?
        if let Some(lower) = &self.lower
            && value < &lower.value
        {
            return &lower.value;
        }
        if let Some(upper) = &self.upper
            && value > &upper.value
        {
            return &upper.value;
        }
        value
    }
}
