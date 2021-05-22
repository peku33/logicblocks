use super::Base;
use crate::{
    util::{
        atomic_cell_erased::{AtomicCellErased, AtomicCellErasedLease},
        waker_stream,
    },
    web::{self, sse_aggregated, uri_cursor},
};
use futures::future::{BoxFuture, FutureExt};
use maplit::hashmap;
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::json;
use std::ops::Deref;

#[derive(Debug)]
struct State<T: Clone + Serialize + Send + Sync + 'static> {
    value_last: Option<T>,
    user_version: usize,
    device_version: usize,
}

#[derive(Debug)]
struct Inner<T: Clone + Serialize + Send + Sync + 'static> {
    state: Mutex<State<T>>,
    sse_aggregated_waker: waker_stream::mpmc::Sender,
}

#[derive(Debug)]
pub struct Property<T: Clone + Serialize + Send + Sync + 'static> {
    inner: AtomicCellErased<Inner<T>>,
}
impl<T: Clone + Serialize + Send + Sync + 'static> Property<T> {
    pub fn new() -> Self {
        let state = State {
            value_last: None,
            user_version: 0,
            device_version: 0,
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
impl<T: Clone + Serialize + Send + Sync + 'static> Base for Property<T> {}
impl<T: Clone + Serialize + Send + Sync + 'static> uri_cursor::Handler for Property<T> {
    fn handle(
        &self,
        request: web::Request,
        uri_cursor: &uri_cursor::UriCursor,
    ) -> BoxFuture<'static, web::Response> {
        match uri_cursor {
            uri_cursor::UriCursor::Terminal => match *request.method() {
                http::Method::GET => {
                    let state = self.inner.state.lock();

                    let device_pending = state.user_version > state.device_version;

                    drop(state);

                    async move {
                        let response = json! {{
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
impl<T: Clone + Serialize + Send + Sync + 'static> sse_aggregated::NodeProvider for Property<T> {
    fn node(&self) -> sse_aggregated::Node {
        sse_aggregated::Node {
            terminal: Some(self.inner.sse_aggregated_waker.receiver_factory()),
            children: hashmap! {},
        }
    }
}

#[derive(Debug)]
pub struct Sink<T: Clone + Serialize + Send + Sync + 'static> {
    inner: AtomicCellErasedLease<Inner<T>>,
}
impl<T: Clone + Serialize + Send + Sync + 'static> Sink<T> {
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

        self.inner.sse_aggregated_waker.wake();

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

        self.property.inner.sse_aggregated_waker.wake();
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
