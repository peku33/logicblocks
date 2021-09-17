use super::Base;
use crate::util::atomic_cell_erased::{AtomicCellErased, AtomicCellErasedLease};
use parking_lot::Mutex;
use serde::Serialize;
use std::ops::Deref;

#[derive(Debug)]
struct State<T>
where
    T: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static,
{
    value: T,
    device_pending: bool,
}

#[derive(Debug)]
struct Inner<T>
where
    T: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static,
{
    state: Mutex<State<T>>,
}

#[derive(Debug)]
pub struct Property<T>
where
    T: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static,
{
    inner: AtomicCellErased<Inner<T>>,
}
impl<T> Property<T>
where
    T: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static,
{
    pub fn new(initial: T) -> Self {
        let state = State {
            value: initial,
            device_pending: true,
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

        if !state.device_pending {
            return None;
        }

        let pending = Pending {
            property: self,
            value: state.value.clone(),
        };

        drop(state);

        Some(pending)
    }
    pub fn device_reset(&self) -> bool {
        let mut state = self.inner.state.lock();

        state.device_pending = true;

        drop(state);

        true
    }
}
impl<T> Base for Property<T> where T: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static {}

#[derive(Debug)]
pub struct Sink<T>
where
    T: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static,
{
    inner: AtomicCellErasedLease<Inner<T>>,
}
impl<T> Sink<T>
where
    T: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static,
{
    fn new(parent: &Property<T>) -> Self {
        let inner = parent.inner.lease();
        Self { inner }
    }

    #[must_use = "use this value to wake properties changed waker"]
    pub fn set(
        &self,
        value: T,
    ) -> bool {
        let mut state = self.inner.state.lock();

        if state.value == value {
            return false;
        }

        state.value = value;
        state.device_pending = true;

        drop(state);

        true
    }

    pub fn peek_last(&self) -> T {
        let state = self.inner.state.lock();

        let value = state.value.clone();

        drop(state);

        value
    }
}

pub struct Pending<'p, T>
where
    T: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static,
{
    property: &'p Property<T>,
    value: T,
}
impl<'p, T> Pending<'p, T>
where
    T: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static,
{
    pub fn commit(self) {
        let mut lock = self.property.inner.state.lock();

        if lock.value == self.value {
            lock.device_pending = false;
        }
    }
}
impl<'p, T> Deref for Pending<'p, T>
where
    T: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static,
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
        let property = Property::new(1usize);
        let sink = property.user_sink();

        // Initial value, no commit
        let pending = property.device_pending().unwrap();
        assert_eq!(*pending, 1);
        assert!(property.device_pending().is_some());

        // Initial value, commit
        let pending = property.device_pending().unwrap();
        assert_eq!(*pending, 1);
        pending.commit();
        assert!(property.device_pending().is_none());

        // No change
        assert_eq!(sink.set(1), false);
        assert!(property.device_pending().is_none());

        // Change
        assert_eq!(sink.set(2), true);
        assert_eq!(sink.set(2), false);
        let pending = property.device_pending().unwrap();
        assert_eq!(*pending, 2);
        pending.commit();
        assert!(property.device_pending().is_none());

        // Two changes
        assert_eq!(sink.set(3), true);
        let pending = property.device_pending().unwrap();
        assert_eq!(*pending, 3);
        assert_eq!(sink.set(4), true);
        assert_eq!(*pending, 3);
        pending.commit();

        let pending = property.device_pending().unwrap();
        assert_eq!(*pending, 4);
        pending.commit();
        assert!(property.device_pending().is_none());
    }
}
