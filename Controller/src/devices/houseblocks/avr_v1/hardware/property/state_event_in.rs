use super::Base;
use crate::util::{
    atomic_cell_erased::{AtomicCellErased, AtomicCellErasedLease},
    waker_stream,
};
use parking_lot::Mutex;
use serde::Serialize;
use std::mem::replace;

#[derive(Debug)]
struct State<S, E>
where
    S: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static,
    E: Clone + Serialize + Send + Sync + 'static,
{
    state: Option<S>,
    events: Vec<E>,
    user_pending: bool,
}

#[derive(Debug)]
struct Inner<S, E>
where
    S: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static,
    E: Clone + Serialize + Send + Sync + 'static,
{
    state: Mutex<State<S, E>>,
    sse_aggregated_waker: waker_stream::mpmc::Sender,
}

#[derive(Debug)]
pub struct Property<S, E>
where
    S: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static,
    E: Clone + Serialize + Send + Sync + 'static,
{
    inner: AtomicCellErased<Inner<S, E>>,
}
impl<S, E> Property<S, E>
where
    S: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static,
    E: Clone + Serialize + Send + Sync + 'static,
{
    pub fn new() -> Self {
        let state = State {
            state: None,
            events: Vec::new(),
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
        let inner_state = self.inner.state.lock();

        let user_pending = inner_state.user_pending;

        drop(inner_state);

        user_pending
    }
    pub fn user_stream(&self) -> Stream<S, E> {
        Stream::new(self)
    }

    // Device
    pub fn device_must_read(&self) -> bool {
        let inner_state = self.inner.state.lock();

        let device_must_read = inner_state.state.is_none();

        drop(inner_state);

        device_must_read
    }
    pub fn device_set(
        &self,
        state: S,
        event: E,
    ) -> bool {
        let mut inner_state = self.inner.state.lock();

        inner_state.state.replace(state);
        inner_state.events.push(event);
        inner_state.user_pending = true;

        drop(inner_state);

        self.inner.sse_aggregated_waker.wake();

        true
    }
    pub fn device_reset(&self) -> bool {
        let mut inner_state = self.inner.state.lock();

        inner_state.state = None;
        inner_state.events.clear();
        inner_state.user_pending = true;

        drop(inner_state);

        self.inner.sse_aggregated_waker.wake();

        true
    }
}
impl<S, E> Base for Property<S, E>
where
    S: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static,
    E: Clone + Serialize + Send + Sync + 'static,
{
}

#[derive(Debug)]
pub struct Stream<S, E>
where
    S: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static,
    E: Clone + Serialize + Send + Sync + 'static,
{
    inner: AtomicCellErasedLease<Inner<S, E>>,
}
impl<S, E> Stream<S, E>
where
    S: PartialEq + Eq + Clone + Serialize + Send + Sync + 'static,
    E: Clone + Serialize + Send + Sync + 'static,
{
    fn new(parent: &Property<S, E>) -> Self {
        let inner = parent.inner.lease();
        Self { inner }
    }

    pub fn take_pending(&self) -> Option<(Option<S>, Box<[E]>)> {
        let mut state_inner = self.inner.state.lock();

        if !state_inner.user_pending {
            return None;
        }

        let state = state_inner.state.clone();
        let events = replace(&mut state_inner.events, Vec::new()).into_boxed_slice();
        state_inner.user_pending = false;

        drop(state_inner);

        self.inner.sse_aggregated_waker.wake();

        Some((state, events))
    }

    pub fn peek_last(&self) -> Option<S> {
        let state_inner = self.inner.state.lock();

        let value = state_inner.state.clone();

        drop(state_inner);

        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1() {
        let property = Property::<[bool; 2], [u8; 2]>::new();
        let stream = property.user_stream();

        assert!(stream.take_pending().is_none());
        assert!(property.device_must_read());

        assert_eq!(property.device_set([false, false], [1, 2]), true);
        assert_eq!(
            stream.take_pending().unwrap(),
            (Some([false, false]), vec![[1, 2]].into_boxed_slice())
        );
        assert!(stream.take_pending().is_none());

        assert!(stream.take_pending().is_none());
        assert!(property.device_reset());
        assert_eq!(
            stream.take_pending().unwrap(),
            (None, vec![].into_boxed_slice())
        );
        assert!(stream.take_pending().is_none());
    }
}
