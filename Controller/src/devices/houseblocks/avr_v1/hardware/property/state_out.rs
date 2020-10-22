use super::Base;
use crate::{
    util::{
        erased_ref::{ErasedRef, ErasedRefLease},
        waker_stream,
    },
    web::{self, sse_aggregated, uri_cursor},
};
use futures::{future::BoxFuture, FutureExt};
use maplit::hashmap;
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::json;
use std::ops::Deref;

#[derive(Debug)]
struct State<T: PartialEq + Clone + Serialize + Send + Sync + 'static> {
    value: T,
    device_pending: bool,
}

#[derive(Debug)]
struct Inner<T: PartialEq + Clone + Serialize + Send + Sync + 'static> {
    state: Mutex<State<T>>,
    sse_aggregated_waker: waker_stream::mpmc::Sender,
}

#[derive(Debug)]
pub struct Property<T: PartialEq + Clone + Serialize + Send + Sync + 'static> {
    inner: ErasedRef<Inner<T>>,
}
impl<T: PartialEq + Clone + Serialize + Send + Sync + 'static> Property<T> {
    pub fn new(initial: T) -> Self {
        let state = State {
            value: initial,
            device_pending: true,
        };
        let state = Mutex::new(state);

        let sse_aggregated_waker = waker_stream::mpmc::Sender::new();

        let inner = Inner {
            state,
            sse_aggregated_waker,
        };
        let inner = ErasedRef::new(inner);

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

        self.inner.sse_aggregated_waker.wake();

        true
    }
}
impl<T: PartialEq + Clone + Serialize + Send + Sync + 'static> Base for Property<T> {}
impl<T: PartialEq + Clone + Serialize + Send + Sync + 'static> uri_cursor::Handler for Property<T> {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        match uri_cursor {
            uri_cursor::UriCursor::Terminal => match *request.method() {
                http::Method::GET => {
                    let state = self.inner.state.lock();

                    let value = state.value.clone();
                    let device_pending = state.device_pending;

                    drop(state);

                    async move {
                        let response = json! {{
                            "value": value,
                            "device_pending": device_pending
                        }};

                        web::Response::ok_json(response)
                    }
                    .boxed()
                }
                _ => async move { web::Response::error_405() }.boxed(),
            },
            _ => async move { web::Response::error_404() }.boxed(),
        }
    }
}
impl<T: PartialEq + Clone + Serialize + Send + Sync + 'static> sse_aggregated::NodeProvider
    for Property<T>
{
    fn node(&self) -> sse_aggregated::Node {
        sse_aggregated::Node {
            terminal: Some(self.inner.sse_aggregated_waker.receiver_factory()),
            children: hashmap! {},
        }
    }
}

#[derive(Debug)]
pub struct Sink<T: PartialEq + Clone + Serialize + Send + Sync + 'static> {
    inner: ErasedRefLease<Inner<T>>,
}
impl<T: PartialEq + Clone + Serialize + Send + Sync + 'static> Sink<T> {
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

        self.inner.sse_aggregated_waker.wake();

        true
    }

    pub fn get_last(&self) -> T {
        let state = self.inner.state.lock();

        let value = state.value.clone();

        drop(state);

        value
    }
}

pub struct Pending<'p, T: PartialEq + Clone + Serialize + Send + Sync + 'static> {
    property: &'p Property<T>,
    value: T,
}
impl<'p, T: PartialEq + Clone + Serialize + Send + Sync + 'static> Pending<'p, T> {
    pub fn commit(self) {
        let mut lock = self.property.inner.state.lock();

        if lock.value == self.value {
            lock.device_pending = false;
        }

        self.property.inner.sse_aggregated_waker.wake();
    }
}
impl<'p, T: PartialEq + Clone + Serialize + Send + Sync + 'static> Deref for Pending<'p, T>
where
    T: Clone + PartialEq,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[cfg(test)]
mod test {
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
