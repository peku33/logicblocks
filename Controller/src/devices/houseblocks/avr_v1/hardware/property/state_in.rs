use super::Base;
use crate::util::atomic_cell_erased::{AtomicCellErased, AtomicCellErasedLease};
use parking_lot::Mutex;

#[derive(Debug)]
struct State<T>
where
    T: Eq + Clone + Send + Sync + 'static,
{
    value: Option<T>,
    user_pending: bool,
}

#[derive(Debug)]
struct Inner<T>
where
    T: Eq + Clone + Send + Sync + 'static,
{
    state: Mutex<State<T>>,
}

#[derive(Debug)]
pub struct Property<T>
where
    T: Eq + Clone + Send + Sync + 'static,
{
    inner: AtomicCellErased<Inner<T>>,
}
impl<T> Property<T>
where
    T: Eq + Clone + Send + Sync + 'static,
{
    pub fn new() -> Self {
        let state = State {
            value: None,
            user_pending: false,
        };
        let state = Mutex::new(state);

        let inner = Inner { state };
        let inner = AtomicCellErased::new(inner);

        Self { inner }
    }

    // User
    pub fn user_pending(&self) -> bool {
        let state = self.inner.state.lock();

        let user_pending = state.user_pending;

        drop(state);

        user_pending
    }
    pub fn user_stream(&self) -> Stream<T> {
        Stream::new(self)
    }

    // Device
    pub fn device_must_read(&self) -> bool {
        let state = self.inner.state.lock();

        let device_must_read = state.value.is_none();

        drop(state);

        device_must_read
    }
    pub fn device_set(
        &self,
        value: T,
    ) -> bool {
        let mut state = self.inner.state.lock();

        if state.value.contains(&value) {
            return false;
        }

        state.value.replace(value);
        state.user_pending = true;

        drop(state);

        true
    }
    pub fn device_reset(&self) -> bool {
        let mut state = self.inner.state.lock();

        if state.value.is_none() {
            return false;
        }

        state.value = None;
        state.user_pending = true;

        drop(state);

        true
    }
}
impl<T> Base for Property<T> where T: Eq + Clone + Send + Sync + 'static {}

#[derive(Debug)]
pub struct Stream<T>
where
    T: Eq + Clone + Send + Sync + 'static,
{
    inner: AtomicCellErasedLease<Inner<T>>,
}
impl<T> Stream<T>
where
    T: Eq + Clone + Send + Sync + 'static,
{
    fn new(parent: &Property<T>) -> Self {
        let inner = parent.inner.lease();
        Self { inner }
    }

    pub fn take_pending(&self) -> Option<Option<T>> {
        let mut state = self.inner.state.lock();

        if !state.user_pending {
            return None;
        }

        let value = state.value.clone();
        state.user_pending = false;

        drop(state);

        Some(value)
    }

    pub fn peek_last(&self) -> Option<T> {
        let state = self.inner.state.lock();

        let value = state.value.clone();

        drop(state);

        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1() {
        let property = Property::<usize>::new();
        let stream = property.user_stream();

        assert!(stream.take_pending().is_none());
        assert!(property.device_must_read());

        assert_eq!(property.device_set(1), true);
        assert_eq!(property.device_set(1), false);
        assert_eq!(stream.take_pending().unwrap().unwrap(), 1);
        assert!(stream.take_pending().is_none());

        assert_eq!(property.device_set(2), true);
        assert_eq!(property.device_set(3), true);
        assert_eq!(stream.take_pending().unwrap().unwrap(), 3);
        assert!(stream.take_pending().is_none());

        assert!(stream.take_pending().is_none());
        assert!(property.device_reset());
        assert!(stream.take_pending().unwrap().is_none());
        assert!(stream.take_pending().is_none());
    }
}
