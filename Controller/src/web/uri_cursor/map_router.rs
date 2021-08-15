use super::{
    super::{Request, Response},
    Handler, UriCursor,
};
use derive_more::Constructor;
use futures::future::{BoxFuture, FutureExt};
use std::collections::HashMap;

#[derive(Constructor)]
pub struct MapRouter<'h> {
    handlers: HashMap<String, &'h (dyn Handler + Sync)>,
}
impl<'h> Handler for MapRouter<'h> {
    fn handle(
        &self,
        request: Request,
        uri_cursor: &UriCursor,
    ) -> BoxFuture<'static, Response> {
        match uri_cursor {
            UriCursor::Terminal => async move { Response::error_404() }.boxed(),
            UriCursor::Next(prefix, uri_cursor) => match self.handlers.get(*prefix) {
                Some(handler) => handler.handle(request, uri_cursor),
                None => async move { Response::error_404() }.boxed(),
            },
        }
    }
}
