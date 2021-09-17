use super::Base;
use crate::util::atomic_cell_erased::{AtomicCellErased, AtomicCellErasedLease};
use parking_lot::Mutex;
use serde::Serialize;
use std::ops::Deref;

#[derive(Debug)]
struct State<T>
where
    T: Clone + Serialize + Send + Sync + 'static,
{
    value_last: Option<T>,
    user_version: usize,
    device_version: usize,
}

#[derive(Debug)]
struct Inner<T>
where
    T: Clone + Serialize + Send + Sync + 'static,
{
    state: Mutex<State<T>>,
}

#[derive(Debug)]
pub struct Property<T>
where
    T: Clone + Serialize + Send + Sync + 'static,
{
    inner: AtomicCellErased<Inner<T>>,
}
impl<T> Property<T>
where
    T: Clone + Serialize + Send + Sync + 'static,
{
    pub fn new() -> Self {
        let state = State {
            value_last: None,
            user_version: 0,
            device_version: 0,
        };
        let state = Mutex::new(state);

        let inner = Inner { state };
        let inner = AtomicCellErased::new(inner);

        Self { inner }
    }

    // User
    pub fn user_sink(&self) -> Sink<T> {
        Sink::new(self)
    }

    // Device
    pub fn device_pending(&self) -> Option<Pending<T>> {
        let state = self.inner.state.lock();

        if state.device_version >= state.user_version {
            return None;
        }

        let value = match state.value_last.as_ref() {
            Some(value) => value.clone(),
            None => return None,
        };

        let pending = Pending {
            property: self,
            value,
            version: state.user_version,
        };

        Some(pending)
    }
}
impl<T> Base for Property<T> where T: Clone + Serialize + Send + Sync + 'static {}

#[derive(Debug)]
pub struct Sink<T>
where
    T: Clone + Serialize + Send + Sync + 'static,
{
    inner: AtomicCellErasedLease<Inner<T>>,
}
impl<T> Sink<T>
where
    T: Clone + Serialize + Send + Sync + 'static,
{
    fn new(parent: &Property<T>) -> Self {
        let inner = parent.inner.lease();
        Self { inner }
    }

    #[must_use = "use this value to wake properties changed waker"]
    pub fn push(
        &self,
        value: T,
    ) -> bool {
        let mut state = self.inner.state.lock();

        state.value_last.replace(value);
        state.user_version += 1;

        drop(state);

        true
    }
}

pub struct Pending<'p, T: Clone + Serialize + Send + Sync + 'static> {
    property: &'p Property<T>,
    value: T,
    version: usize,
}
impl<'p, T: Clone + Serialize + Send + Sync + 'static> Pending<'p, T> {
    pub fn commit(self) {
        let mut state = self.property.inner.state.lock();

        state.device_version = self.version;

        drop(state);
    }
}
impl<'p, T> Deref for Pending<'p, T>
where
    T: Clone + Serialize + Send + Sync + 'static,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1() {
        let property = Property::new();
        let sink = property.user_sink();

        // Initial state
        assert!(property.device_pending().is_none());

        // Sink 1
        assert_eq!(sink.push(1usize), true);
        let pending = property.device_pending().unwrap();
        assert_eq!(*pending, 1);
        pending.commit();
        assert!(property.device_pending().is_none());

        // Sink 2
        assert_eq!(sink.push(2usize), true);
        let pending = property.device_pending().unwrap();
        assert_eq!(*pending, 2);
        let pending = property.device_pending().unwrap();
        assert_eq!(*pending, 2);
        pending.commit();
        assert!(property.device_pending().is_none());

        // Sink 3
        assert_eq!(sink.push(3usize), true);
        let pending = property.device_pending().unwrap();
        assert_eq!(*pending, 3);
        assert_eq!(sink.push(4usize), true);
        assert_eq!(*pending, 3);
        pending.commit();
        let pending = property.device_pending().unwrap();
        assert_eq!(*pending, 4);
        pending.commit();
        assert!(property.device_pending().is_none());
    }
}
