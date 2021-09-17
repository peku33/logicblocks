use super::Base;
use crate::util::{
    atomic_cell_erased::{AtomicCellErased, AtomicCellErasedLease},
    waker_stream,
};
use parking_lot::Mutex;
use serde::Serialize;

#[derive(Debug)]
struct State<T: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static> {
    value: Option<T>,
    user_pending: bool,
}

#[derive(Debug)]
struct Inner<T: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static> {
    state: Mutex<State<T>>,
    sse_aggregated_waker: waker_stream::mpmc::Sender,
}

#[derive(Debug)]
pub struct Property<T: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static> {
    inner: AtomicCellErased<Inner<T>>,
}
impl<T: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static> Property<T> {
    pub fn new() -> Self {
        let state = State {
            value: None,
            user_pending: false,
        };
        let state = Mutex::new(state);

        let sse_aggregated_waker = waker_stream::mpmc::Sender::new();
        let inner = Inner {
            state,
            sse_aggregated_waker,
        };
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

        self.inner.sse_aggregated_waker.wake();

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

        self.inner.sse_aggregated_waker.wake();

        true
    }
}
impl<T: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static> Base for Property<T> {}

#[derive(Debug)]
pub struct Stream<T: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static> {
    inner: AtomicCellErasedLease<Inner<T>>,
}
impl<T: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static> Stream<T> {
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

        self.inner.sse_aggregated_waker.wake();

        Some(value)
    }

    pub fn get_last(&self) -> Option<T> {
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
