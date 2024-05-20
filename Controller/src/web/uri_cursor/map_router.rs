use super::{
    super::{Request, Response},
    Handler, UriCursor,
};
use futures::future::{BoxFuture, FutureExt};
use std::collections::HashMap;

// #[derive(Debug)] // Debug not possible
pub struct MapRouter<'h> {
    handlers: HashMap<String, &'h (dyn Handler + Sync)>,
}
impl<'h> MapRouter<'h> {
    pub fn new(handlers: HashMap<String, &'h (dyn Handler + Sync)>) -> Self {
        Self { handlers }
    }
}
impl<'h> Handler for MapRouter<'h> {
    fn handle(
        &self,
        request: Request,
        uri_cursor: &UriCursor,
    ) -> BoxFuture<'static, Response> {
        match uri_cursor {
            UriCursor::Terminal => async { Response::error_404() }.boxed(),
            UriCursor::Next(prefix, uri_cursor) => match self.handlers.get(*prefix) {
                Some(handler) => handler.handle(request, uri_cursor),
                None => async { Response::error_404() }.boxed(),
            },
        }
    }
}
