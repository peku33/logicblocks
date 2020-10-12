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

#[derive(Debug)]
struct State<T: PartialEq + Clone + Serialize + Send + Sync + 'static> {
    value: Option<T>,
    user_pending: bool,
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
        let inner = ErasedRef::new(inner);

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
                    let user_pending = state.user_pending;

                    drop(state);

                    async move {
                        let response = json! {{
                            "value": value,
                            "user_pending": user_pending
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
pub struct Stream<T: PartialEq + Clone + Serialize + Send + Sync + 'static> {
    inner: ErasedRefLease<Inner<T>>,
}
impl<T: PartialEq + Clone + Serialize + Send + Sync + 'static> Stream<T> {
    fn new(parent: &Property<T>) -> Self {
        let inner = parent.inner.lease();
        Self { inner }
    }

    pub fn take(&self) -> Option<Option<T>> {
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
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_1() {
        let property = Property::<usize>::new();
        let stream = property.user_stream();

        assert!(stream.take().is_none());
        assert!(property.device_must_read());

        assert_eq!(property.device_set(1), true);
        assert_eq!(property.device_set(1), false);
        assert_eq!(stream.take().unwrap().unwrap(), 1);
        assert!(stream.take().is_none());

        assert_eq!(property.device_set(2), true);
        assert_eq!(property.device_set(3), true);
        assert_eq!(stream.take().unwrap().unwrap(), 3);
        assert!(stream.take().is_none());

        assert!(stream.take().is_none());
        assert!(property.device_reset());
        assert!(stream.take().unwrap().is_none());
        assert!(stream.take().is_none());
    }
}
