use super::super::{Request, Response};
use super::{Handler, UriCursor};
use futures::future::{ready, BoxFuture};
use futures::FutureExt;
use std::collections::HashMap;

pub struct NextItemMap<'a> {
    inner: HashMap<String, &'a (dyn Handler + Sync + Send)>,
}
impl<'a> NextItemMap<'a> {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn set(
        &mut self,
        next_item: String,
        handler: &'a (dyn Handler + Sync + Send),
    ) {
        self.inner.insert(next_item, handler);
    }
}
impl<'a> Handler for NextItemMap<'a> {
    fn handle(
        &self,
        request: Request,
        uri_cursor: UriCursor,
    ) -> BoxFuture<'static, Response> {
        let (next_item, uri_cursor) = uri_cursor.next_item();
        let uri_cursor = match uri_cursor {
            Some(uri_cursor) => uri_cursor,
            None => return ready(Response::error_404()).boxed(),
        };
        let item = self.inner.get(next_item);
        match item {
            Some(next_routed_handler) => next_routed_handler.handle(request, uri_cursor),
            None => ready(Response::error_404()).boxed(),
        }
    }
}
