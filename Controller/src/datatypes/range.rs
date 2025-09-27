use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct RangeBoundary<V> {
    pub value: V,
    pub inclusive: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct Range<V> {
    pub lower: Option<RangeBoundary<V>>,
    pub upper: Option<RangeBoundary<V>>,
}
impl<V> Range<V>
where
    V: PartialOrd,
{
    pub fn contains(
        &self,
        value: &V,
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
impl<V> Range<V>
where
    V: Clone + PartialOrd,
{
    pub fn clamp_to(
        &self,
        value: V,
    ) -> V {
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
