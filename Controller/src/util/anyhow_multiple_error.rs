use anyhow::Error as AnyhowError;
use std::{error::Error as StdError, fmt};

// #[derive(Debug)] implemented manually
pub struct AnyhowMultipleError {
    inner: Box<[AnyhowError]>,
}
impl AnyhowMultipleError {
    pub fn new(inner: Box<[AnyhowError]>) -> Self {
        Self { inner }
    }
}
impl FromIterator<AnyhowError> for AnyhowMultipleError {
    fn from_iter<T: IntoIterator<Item = AnyhowError>>(iter: T) -> Self {
        Self {
            inner: iter.into_iter().collect::<Box<[_]>>(),
        }
    }
}
impl fmt::Debug for AnyhowMultipleError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_list().entries(self.inner.iter()).finish()
    }
}
impl fmt::Display for AnyhowMultipleError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_list().entries(self.inner.iter()).finish()
    }
}
impl StdError for AnyhowMultipleError {}
